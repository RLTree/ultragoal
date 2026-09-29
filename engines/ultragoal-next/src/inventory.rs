use super::*;
use fs_adapter::{Capture, Identity, Kind, Limits, Listing, Problem, Root};
use std::collections::BTreeSet;
use std::sync::Arc;
mod large;

/// The main evaluation frame and the whole-scope key are planned to this size before
/// capture. The margin covers rows known only after capture: problem rows, content
/// escapes and chunk counts.
pub const FRAME_BUDGET: usize = MAX - 512 * 1024;
const OMITTED: &str = "evaluation frame bound";

pub struct Session {
    root: Arc<Root>,
    limits: Limits,
    exclusions: Vec<Vec<u8>>,
}
pub enum Baseline {
    Checked(Arc<fs_adapter::MembershipCheckpoint>),
    Uncached(Listing),
}
impl Baseline {
    pub fn listing(&self) -> &Listing {
        match self {
            Self::Checked(c) => c.listing(),
            Self::Uncached(l) => l,
        }
    }
}
pub struct Observation {
    pub files: BTreeMap<String, Vec<u8>>,
    pub identities: BTreeMap<String, Identity>,
    pub kinds: BTreeMap<String, String>,
    pub problems: BTreeMap<String, (String, String)>,
    pub issues: Vec<(String, String)>,
    pub complete: bool,
    pub root_identity: Option<Identity>,
    pub exclusions: Vec<String>,
}
fn bounded<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    let started = Instant::now();
    loop {
        if is_interrupted() {
            return Err("snapshot observation cancelled".into());
        }
        match rx.recv_timeout(Duration::from_millis(20)) {
            Ok(v) => return Ok(v),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err("snapshot observer stopped".into());
            }
            Err(_) => {
                if started.elapsed() > Duration::from_secs(30) {
                    return Err("snapshot observation deadline exceeded".into());
                }
            }
        }
    }
}
impl Session {
    pub fn open(path: &Path) -> Result<Self, String> {
        let rows = evaluate("EXCLUSIONS\n".into())?;
        let mut exclusions = Vec::new();
        for r in rows {
            if r.len() != 2 || r[0] != "EXCLUDE" {
                return Err("Bend exclusion policy shape".into());
            }
            exclusions.push(r[1].as_bytes().to_vec());
        }
        Ok(Self {
            root: Arc::new(Root::open(path).map_err(|e| e.to_string())?),
            // The observer checks host headroom while the listing grows.
            limits: Limits {
                max_entries: usize::MAX,
                max_depth: usize::MAX,
                max_path_bytes: 4096,
                max_file_bytes: MAX,
                max_total_bytes: MAX - 1024 * 1024,
            },
            exclusions,
        })
    }
    pub fn enumerate(&self) -> Result<Listing, String> {
        let root = self.root.clone();
        let limits = self.limits;
        let excluded = self.exclusions.clone();
        bounded(move || root.enumerate_excluding(&limits, &excluded))
    }
    pub fn baseline(&self) -> Result<Baseline, String> {
        let root = self.root.clone();
        let limits = self.limits;
        let excluded = self.exclusions.clone();
        match bounded(move || root.membership_checkpoint_excluding(&limits, &excluded))? {
            Ok(checkpoint) => Ok(Baseline::Checked(Arc::new(checkpoint))),
            Err(fs_adapter::MembershipFallback::InitialListingIncomplete(listing)) => {
                Ok(Baseline::Uncached(*listing))
            }
            Err(_) => Ok(Baseline::Uncached(self.enumerate()?)),
        }
    }
    /// Current identities of the selected files. Content is never reread: the
    /// comparison is by identity, which every write changes.
    pub fn revalidate(
        &self,
        baseline: &Baseline,
        wanted: &BTreeSet<String>,
    ) -> Result<(Observation, String), String> {
        if let Baseline::Checked(checkpoint) = baseline {
            let root = self.root.clone();
            let limits = self.limits;
            let exclusions = self.exclusions.clone();
            let held = checkpoint.clone();
            if bounded(move || root.revalidate_membership(&held, &limits, &exclusions))?.is_ok() {
                let observed = self.identities(checkpoint.listing(), wanted)?;
                let root = self.root.clone();
                let limits = self.limits;
                let exclusions = self.exclusions.clone();
                let held = checkpoint.clone();
                if bounded(move || root.revalidate_membership(&held, &limits, &exclusions))?.is_ok()
                {
                    return Ok((observed,"directory identities revalidated; selected file identities rechecked under window lineage checks; metadata observation only".into()));
                }
            }
        }
        Ok((
            self.identities(&self.enumerate()?, wanted)?,
            "full enumeration fallback; selected file identities rechecked; metadata observation only".into(),
        ))
    }
    pub fn capture(
        &self,
        listing: &Listing,
        wanted: Option<&BTreeSet<String>>,
    ) -> Result<Observation, String> {
        let captures = if let Some(w) = wanted {
            let paths = w.iter().map(|p| p.as_bytes().to_vec()).collect::<Vec<_>>();
            let root = self.root.clone();
            let limits = self.limits;
            let excluded = self.exclusions.clone();
            if limits.max_total_bytes <= MAX {
                bounded(move || root.capture_selected_excluding(&paths, &limits, &excluded))?
            } else {
                let batches = batches(listing, paths);
                bounded(move || {
                    let mut captured = Vec::new();
                    let mut total = 0usize;
                    let chunk_limits = Limits {
                        max_total_bytes: MAX - 1024 * 1024,
                        ..limits
                    };
                    for batch in batches {
                        let batch = batch?;
                        if is_interrupted() {
                            return Err("content capture cancelled".to_string());
                        }
                        let mut part =
                            root.capture_selected_excluding(&batch, &chunk_limits, &excluded);
                        total = total.saturating_add(
                            part.iter()
                                .filter_map(|c| c.bytes.as_ref())
                                .map(Vec::len)
                                .sum::<usize>(),
                        );
                        if total > limits.max_total_bytes {
                            return Err(format!(
                                "selected content exceeds {} MiB operation bound",
                                limits.max_total_bytes >> 20
                            ));
                        }
                        captured.append(&mut part);
                    }
                    Ok(captured)
                })??
            }
        } else {
            Vec::new()
        };
        project(listing, captures, self.limits.max_file_bytes)
    }
    /// Captures `wanted` in listing-size batches of about 8 MiB and hands `each` every
    /// file member of `observed` in path order with its admitted bytes (empty when not
    /// captured), so at most one batch of content is held at a time.
    pub fn stream(
        &self,
        listing: &Listing,
        observed: &mut Observation,
        wanted: &BTreeSet<String>,
        mut each: impl FnMut(&mut Observation, &str, Vec<u8>) -> Result<(), String>,
    ) -> Result<(), String> {
        // A batch is at most 8 MiB by listing size or one file within the file bound.
        let limits = Limits { max_total_bytes: MAX, ..self.limits };
        let paths: Vec<String> = observed.files.keys().cloned().collect();
        let mut next = paths.iter().peekable();
        for batch in batches(listing, wanted.iter().map(|p| p.as_bytes().to_vec()).collect()) {
            let batch = batch?;
            if is_interrupted() {
                return Err("content capture cancelled".into());
            }
            let end = String::from_utf8(batch.last().cloned().unwrap_or_default())
                .map_err(|_| "capture path UTF-8")?;
            let mut held = BTreeMap::new();
            if batch.len() == 1 && listing.members.iter().any(|m|
                m.path == batch[0] && m.identity.size > (MAX / 3) as u64)
            {
                if let Some((path, bytes)) = self.capture_large(&batch[0], observed) {
                    held.insert(path, bytes);
                }
            } else {
                let (root, excluded) = (self.root.clone(), self.exclusions.clone());
                for capture in bounded(move || root.capture_selected_excluding(&batch, &limits, &excluded))? {
                    if let Some((path, bytes)) = observed.admit(capture, limits.max_file_bytes) {
                        held.insert(path, bytes);
                    }
                }
            }
            while let Some(path) = next.next_if(|p| p.as_str() <= end.as_str()) {
                each(observed, path, held.remove(path).unwrap_or_default())?;
            }
        }
        for path in next {
            each(observed, path, Vec::new())?;
        }
        Ok(())
    }
    /// One selected logical source under descriptor custody. The producer
    /// hashes before BEGIN, then delivers bounded pages with synchronous
    /// backpressure; the final path/descriptor validation follows END's data.
    pub fn stream_source(
        &self,
        path: &str,
        begin: impl FnMut(&fs_adapter::StreamedSource) -> Result<(), String>,
        each: impl FnMut(u64, &str) -> Result<(), String>,
    ) -> Result<fs_adapter::StreamedSource, fs_adapter::StreamError> {
        if path.split('/').any(|part| self.exclusions.iter().any(|x| x == part.as_bytes())) {
            return Err(fs_adapter::StreamError::InvalidPath);
        }
        self.root.stream_source(path.as_bytes(), begin, each)
    }
    /// Current identities of the selected files, content not reread.
    pub fn identities(
        &self,
        listing: &Listing,
        wanted: &BTreeSet<String>,
    ) -> Result<Observation, String> {
        let paths = wanted.iter().map(|p| p.as_bytes().to_vec()).collect::<Vec<_>>();
        let root = self.root.clone();
        let limits = self.limits;
        let excluded = self.exclusions.clone();
        project(
            listing,
            bounded(move || root.identities_selected_excluding(&paths, &limits, &excluded))?,
            limits.max_file_bytes,
        )
    }
    pub fn observe(&self, wanted: Option<&BTreeSet<String>>) -> Result<Observation, String> {
        self.capture(&self.enumerate()?, wanted)
    }
}
/// Selected paths in batches sized to current headroom and the provider frame.
struct Batches {
    sizes: BTreeMap<Vec<u8>, u64>,
    paths: std::vec::IntoIter<Vec<u8>>,
    next: Option<Vec<u8>>,
}
impl Iterator for Batches {
    type Item = Result<Vec<Vec<u8>>, String>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_none() && self.paths.len() == 0 { return None; }
        let budget = match batch_budget() { Ok(v) => v, Err(e) => return Some(Err(e)) };
        let mut batch = Vec::new();
        let mut bytes = 0u64;
        while let Some(path) = self.next.take().or_else(|| self.paths.next()) {
            let size = self.sizes.get(&path).copied().unwrap_or(0);
            if !batch.is_empty() && bytes.saturating_add(size) > budget {
                self.next = Some(path);
                break;
            }
            bytes = bytes.saturating_add(size);
            batch.push(path);
        }
        (!batch.is_empty()).then_some(Ok(batch))
    }
}
fn batches(listing: &Listing, paths: Vec<Vec<u8>>) -> Batches {
    Batches { sizes: listing.members.iter().map(|m| (m.path.clone(), m.identity.size)).collect(), paths: paths.into_iter(), next: None }
}
fn batch_budget() -> Result<u64, String> {
    let headroom = crate::resources::current().ok_or("capture headroom unavailable; retry after resource telemetry recovers")?;
    if headroom.work_bytes() == 0 { return Err("capture paused by current memory pressure; retry after headroom recovers".into()); }
    Ok(headroom.frame_bytes(MAX).min(MAX / 2) as u64)
}
fn project(listing: &Listing, captures: Vec<Capture>, max_file: usize) -> Result<Observation, String> {
    let mut out = Observation::listed(listing)?;
    for capture in captures {
        if let Some((path, bytes)) = out.admit(capture, max_file) {
            out.files.insert(path, bytes);
        }
    }
    Ok(out)
}
/// Whether `path` lies in one of the omitted units.
#[cfg(test)]
pub fn within(units: &[String], path: &str) -> bool {
    units.iter().any(|u| path.strip_prefix(u.as_str()).is_some_and(|rest| rest.is_empty() || rest.starts_with('/')))
}
/// Frame and key bytes a member contributes, from its listing size.
#[derive(Clone, Copy, Default)]
struct Cost {
    frame: usize,
    key: usize,
    chunk: usize,
    rows: usize,
}
impl Cost {
    fn plus(self, o: Self) -> Self {
        Self { frame: self.frame + o.frame, key: self.key + o.key, chunk: self.chunk + o.chunk, rows: self.rows + o.rows }
    }
    fn minus(self, o: Self) -> Self {
        Self { frame: self.frame - o.frame, key: self.key - o.key, chunk: self.chunk - o.chunk, rows: self.rows - o.rows }
    }
}
/// What the main evaluation leaves out, and its estimated frame and key bytes.
pub struct FramePlan {
    pub omitted: Vec<String>,
    pub frame: usize,
    pub key: usize,
}
/// Rows the main frame carries for one selection, as planned before capture.
pub struct FrameShape<'a> {
    pub wanted: &'a BTreeSet<String>,
    pub native: &'a BTreeSet<String>,
    pub kept: &'a BTreeSet<String>,
    pub chunked: bool,
    /// Contract, operation and budget rows.
    pub fixed: usize,
    /// Partial rows per chunk.
    pub partials: usize,
}
impl Observation {
    fn listed(listing: &Listing) -> Result<Self, String> {
        let mut out = Observation {
            files: BTreeMap::new(),
            identities: BTreeMap::new(),
            kinds: BTreeMap::new(),
            problems: BTreeMap::new(),
            issues: Vec::new(),
            complete: listing.complete,
            root_identity: listing.root_identity,
            exclusions: listing
                .excluded_basenames
                .iter()
                .map(|x| String::from_utf8(x.clone()).map_err(|_| "exclusion UTF-8".to_string()))
                .collect::<Result<_, _>>()?,
        };
        for issue in &listing.issues {
            out.issues.push((
                String::from_utf8(issue.path.clone()).unwrap_or_default(),
                match issue.problem {
                    Problem::EntryBound => "listing entry bound".into(),
                    p => format!("{p:?}"),
                },
            ));
        }
        if !out.complete && out.issues.is_empty() {
            out.issues.push(("".into(), "incomplete listing".into()));
        }
        for member in &listing.members {
            let Ok(path) = String::from_utf8(member.path.clone()) else {
                out.complete = false;
                out.issues.push((
                    "".into(),
                    "non-UTF8 member not representable in protocol".into(),
                ));
                continue;
            };
            let kind = match member.kind {
                Kind::File => "file",
                Kind::Symlink => "symlink",
                Kind::Other => "other",
                Kind::Directory => "directory",
            }
            .to_string();
            out.kinds.insert(path.clone(), kind.clone());
            out.identities.insert(path.clone(), member.identity);
            if member.kind == Kind::File {
                out.files.insert(path, Vec::new());
            } else if member.kind != Kind::Directory {
                out.problems
                    .insert(path, (kind, "unsupported source kind".into()));
            }
        }
        Ok(out)
    }
    /// Records a capture's problem or identity; returns the admitted bytes.
    fn admit(&mut self, capture: Capture, max_file: usize) -> Option<(String, Vec<u8>)> {
        let Ok(path) = String::from_utf8(capture.path) else {
            self.complete = false;
            self.issues.push(("".into(), "non-UTF8 capture".into()));
            return None;
        };
        let mut problem = capture.problem;
        if let Some(identity) = capture.identity
            && self.identities.get(&path) != Some(&identity)
        {
            problem = Some(Problem::Unstable);
        }
        if let Some(p) = problem {
            if path.is_empty() {
                self.complete = false;
                self.issues.push((path, format!("{p:?}")));
            } else {
                let kind = self.kinds.get(&path).cloned().unwrap_or("unknown".into());
                let reason = match p {
                    Problem::Limit if self.identities.get(&path).is_some_and(|i| i.size > max_file as u64) => {
                        "file byte bound".into()
                    }
                    Problem::Limit => "operation byte bound".into(),
                    p => format!("{p:?}"),
                };
                self.problems.insert(path, (kind, reason));
            }
            return None;
        }
        let (Some(bytes), Some(identity)) = (capture.bytes, capture.identity) else {
            return None;
        };
        if std::str::from_utf8(&bytes).is_err() {
            self.problems
                .insert(path, ("file".into(), "invalid UTF-8".into()));
            return None;
        }
        self.identities.insert(path.clone(), identity);
        Some((path, bytes))
    }
    pub fn digest(&self, path: &str, wanted: &BTreeSet<String>, bytes: &[u8]) -> String {
        if self.problems.contains_key(path) {
            "unavailable".into()
        } else if wanted.contains(path) {
            hash(bytes)
        } else {
            "unread".into()
        }
    }
    pub fn frame(&self, wanted: &BTreeSet<String>) -> String {
        let mut frame = self.head();
        for (path, bytes) in &self.files {
            frame += &row(&["F", path, &self.digest(path, wanted, bytes), std::str::from_utf8(bytes).unwrap_or("")]);
        }
        frame + &self.tail()
    }
    /// Exclusion and directory rows, which precede the F rows.
    pub fn head(&self) -> String {
        let mut frame = String::new();
        for x in &self.exclusions {
            frame += &row(&["X", x]);
        }
        for (path, kind) in &self.kinds {
            if kind == "directory" {
                frame += &row(&["D", path]);
            }
        }
        frame
    }
    /// Problem and issue rows, which follow the F rows.
    pub fn tail(&self) -> String {
        let mut frame = String::new();
        for (path, (kind, reason)) in &self.problems {
            frame += &row(&["U", path, kind, reason]);
        }
        for (path, reason) in &self.issues {
            frame += &row(&["I", path, reason]);
        }
        frame
    }
    fn cost(&self, path: &str, kind: &str, shape: &FrameShape) -> Cost {
        let p = enc_len(path);
        match kind {
            "directory" => Cost { frame: p + 3, key: p + 11, ..Cost::default() },
            "file" => {
                let wanted = shape.wanted.contains(path);
                let digest = if wanted { 64 } else { 6 };
                let size = self.identities.get(path).map_or(0, |i| i.size as usize);
                // Chunked: every file is one CF row (path, digest) from the chunk core, and
                // only a kept file also has its F row with content in the main frame.
                let row = if !shape.chunked {
                    p + digest + if wanted { size } else { 0 } + 5
                } else if shape.kept.contains(path) {
                    2 * (p + digest + 5) + size
                } else {
                    p + digest + 5
                };
                let native = if shape.native.contains(path) { p + 187 } else { 0 };
                Cost {
                    frame: row + native,
                    key: p + digest + 2,
                    chunk: if shape.chunked { p + digest + 5 + if wanted { size } else { 0 } } else { 0 },
                    rows: usize::from(shape.chunked),
                }
            }
            kind => Cost { frame: p + kind.len() + 28, key: p + kind.len() + 2, ..Cost::default() },
        }
    }
    /// Direct children of `prefix` ("" for the root) in path order, each with its own
    /// cost and the cost of everything beneath it.
    fn units(&self, prefix: &str, shape: &FrameShape) -> Vec<(String, Cost, Cost, bool)> {
        let mut units: BTreeMap<&str, (Cost, Cost, bool)> = BTreeMap::new();
        for (path, kind) in self.kinds.range(prefix.to_string()..).take_while(|(p, _)| p.starts_with(prefix)) {
            let rest = &path[prefix.len()..];
            let cost = self.cost(path, kind, shape);
            match rest.find('/') {
                None => {
                    let unit = units.entry(path).or_default();
                    unit.0 = cost;
                    unit.2 = kind == "directory";
                }
                Some(i) => {
                    let unit = units.entry(&path[..prefix.len() + i]).or_default();
                    unit.1 = unit.1.plus(cost);
                }
            }
        }
        units.into_iter().map(|(u, (own, below, dir))| (u.to_string(), own, below, dir)).collect()
    }
    /// Subtrees left out so that the main evaluation frame and the whole-scope key fit
    /// `budget`: trailing top-level entries in path order and, inside the last one that
    /// must go, its own trailing entries, as deep as the rest still fits.
    pub fn plan_frame(&self, shape: &FrameShape, budget: usize) -> FramePlan {
        let fixed = shape.fixed
            + self.exclusions.iter().map(|x| enc_len(x) + 3).sum::<usize>()
            + self.issues.iter().map(|(p, r)| enc_len(p) + r.len() + 4).sum::<usize>();
        let keyed = 256 + self.exclusions.iter().map(|x| enc_len(x) + 3).sum::<usize>();
        let sizes = |c: Cost| {
            let chunks = if shape.chunked {
                c.chunk.div_ceil(chunks::CHUNK_BYTES).max(c.rows.div_ceil(chunks::FRAME_ROWS)).max(1)
            } else {
                0
            };
            (c.frame + fixed + chunks * (38 + shape.partials), c.key + keyed)
        };
        let fits = |c: Cost| {
            let (frame, key) = sizes(c);
            frame <= budget && key <= budget
        };
        let named = |unit: &str| Cost { frame: enc_len(unit) + OMITTED.len() + 4, ..Cost::default() };
        let mut total = self.kinds.iter().fold(Cost::default(), |t, (p, k)| t.plus(self.cost(p, k, shape)));
        let mut omitted = Vec::new();
        let mut saved: Option<(Vec<String>, Cost)> = None;
        let mut prefix = String::new();
        while !fits(total) {
            let mut last = None;
            for (unit, own, below, dir) in self.units(&prefix, shape).into_iter().rev() {
                if fits(total) {
                    break;
                }
                total = total.minus(if dir { below } else { own.plus(below) }).plus(named(&unit));
                omitted.push(unit.clone());
                last = Some((unit, below, dir));
            }
            if !fits(total) {
                if let Some((o, t)) = saved.take() {
                    (omitted, total) = (o, t);
                }
                break;
            }
            let Some((unit, below, true)) = last else { break };
            saved = Some((omitted.clone(), total));
            omitted.pop();
            total = total.plus(below).minus(named(&unit));
            prefix = unit + "/";
        }
        let (frame, key) = sizes(total);
        omitted.sort();
        FramePlan { omitted, frame, key }
    }
    /// Leaves each unit (a directory's contents, or a file) out of this observation
    /// and names it as an issue.
    pub fn omit(&mut self, units: &[String]) {
        if units.is_empty() {
            return;
        }
        let dirs: BTreeSet<&str> = units.iter().map(String::as_str).filter(|u| self.kinds.get(*u).is_some_and(|k| k == "directory")).collect();
        let files: BTreeSet<&str> = units.iter().map(String::as_str).filter(|u| !dirs.contains(u)).collect();
        let gone = |p: &str| files.contains(p) || p.match_indices('/').any(|(i, _)| dirs.contains(&p[..i]));
        self.kinds.retain(|p, _| !gone(p));
        self.identities.retain(|p, _| !gone(p));
        self.files.retain(|p, _| !gone(p));
        self.problems.retain(|p, _| !gone(p));
        self.issues.retain(|(p, _)| !gone(p));
        self.issues.extend(units.iter().map(|u| (u.clone(), OMITTED.to_string())));
        self.complete = false;
    }
    pub fn changes(&self, new: &Self) -> Vec<(String, String)> {
        let mut changes = Vec::new();
        let keys: BTreeSet<_> = self.kinds.keys().chain(new.kinds.keys()).collect();
        for path in keys {
            if self.kinds.get(path) != new.kinds.get(path) {
                changes.push((path.clone(), "membership".into()));
            } else if self.kinds.get(path).is_some_and(|k| k != "directory")
                && self.identities.get(path) != new.identities.get(path)
            {
                changes.push((path.clone(), "content".into()));
            }
        }
        let same_root = match (self.root_identity, new.root_identity) {
            (Some(a), Some(b)) => a.device == b.device && a.inode == b.inode,
            _ => false,
        };
        if !same_root || self.exclusions != new.exclusions {
            changes.push(("".into(), "root-or-universe".into()));
        }
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    struct Tree(PathBuf);
    impl Tree {
        fn new(files: &[(&str, usize)]) -> Self {
            let root = env::temp_dir().join(format!("ug-inventory-{}", op_id().unwrap()));
            for (path, size) in files {
                let path = root.join(path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, "x".repeat(*size)).unwrap();
            }
            Self(fs::canonicalize(root).unwrap())
        }
        fn session(&self, limits: Limits) -> Session {
            Session { root: Arc::new(Root::open(&self.0).unwrap()), limits, exclusions: Vec::new() }
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn limits(max_entries: usize, max_file_bytes: usize, max_total_bytes: usize) -> Limits {
        Limits { max_entries, max_depth: 64, max_path_bytes: 4096, max_file_bytes, max_total_bytes }
    }
    fn set(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn incomplete_listing_revalidates_identities_without_rereading_content() {
        let tree = Tree::new(&[("a.txt", 8), ("b.txt", 8), ("c.txt", 8), ("d/e.txt", 1), ("d/f.txt", 1)]);
        let session = tree.session(limits(4, MAX, 10));
        let baseline = session.baseline().unwrap();
        assert!(matches!(baseline, Baseline::Uncached(_)) && !baseline.listing().complete);
        let wanted = set(&["a.txt", "b.txt", "c.txt"]);
        let (current, mode) = session.revalidate(&baseline, &wanted).unwrap();
        assert!(mode.starts_with("full enumeration fallback"), "{mode}");
        assert!(current.problems.is_empty(), "{:?}", current.problems);
        let listed = Observation::listed(baseline.listing()).unwrap();
        assert!(wanted.iter().all(|p| current.identities.get(p) == listed.identities.get(p)));
        assert!(current.issues.contains(&("d".into(), "listing entry bound".into())));
        // Recapturing content under the same total bound, as the fallback did, loses files.
        let recaptured = session.observe(Some(&wanted)).unwrap();
        assert_eq!(recaptured.problems.get("b.txt").map(|p| p.1.as_str()), Some("operation byte bound"));
    }

    #[test]
    fn problem_reasons_name_the_bound_that_applied() {
        let tree = Tree::new(&[("a.txt", 8), ("b.txt", 8), ("big.txt", 64), ("d/e.txt", 1), ("d/f.txt", 1)]);
        let session = tree.session(limits(5, 32, 12));
        let listing = session.enumerate().unwrap();
        let observed = session.capture(&listing, Some(&set(&["a.txt", "b.txt", "big.txt"]))).unwrap();
        let reasons: Vec<(&str, &str)> = observed.problems.iter().map(|(p, r)| (p.as_str(), r.1.as_str())).collect();
        assert_eq!(reasons, [("b.txt", "operation byte bound"), ("big.txt", "file byte bound")]);
        assert_eq!(observed.files["a.txt"].len(), 8);
        assert_eq!(observed.issues, [("d".to_string(), "listing entry bound".to_string())]);
    }

    fn fixture() -> Tree {
        let mut files = Vec::new();
        for dir in ["a", "b", "c", "d"] {
            for i in 0..10 {
                files.push((format!("{dir}/f{i:02}.txt"), 1000));
            }
        }
        files.push(("README.txt".into(), 10));
        let files: Vec<(&str, usize)> = files.iter().map(|(p, s)| (p.as_str(), *s)).collect();
        Tree::new(&files)
    }

    #[test]
    fn frame_budget_names_trailing_directories_and_they_are_not_captured() {
        let tree = fixture();
        let session = tree.session(limits(1000, MAX, MAX));
        let listing = session.enumerate().unwrap();
        let mut observed = Observation::listed(&listing).unwrap();
        let wanted: BTreeSet<String> = observed.files.keys().cloned().collect();
        let empty = BTreeSet::new();
        let shape = |chunked| FrameShape { wanted: &wanted, native: &empty, kept: &empty, chunked, fixed: 100, partials: 60 };
        let whole = observed.plan_frame(&shape(true), usize::MAX);
        assert!(whole.omitted.is_empty());
        let without = |dirs: &[&str]| {
            let mut o = Observation::listed(&listing).unwrap();
            o.omit(&dirs.iter().map(|d| d.to_string()).collect::<Vec<_>>());
            let kept = wanted.iter().filter(|p| !within(&dirs.iter().map(|d| d.to_string()).collect::<Vec<_>>(), p)).cloned().collect();
            o.plan_frame(&FrameShape { wanted: &kept, ..shape(true) }, usize::MAX).frame
        };
        // Exactly what remains without c and d: nothing of c fits back in.
        let plan = observed.plan_frame(&shape(true), without(&["c", "d"]));
        assert_eq!(plan.omitted, ["c", "d"]);
        // Room for part of c (all but its last five files): its trailing files are named, d stays out whole.
        let plan = observed.plan_frame(&shape(true), without(&["c/f05.txt", "c/f06.txt", "c/f07.txt", "c/f08.txt", "c/f09.txt", "d"]));
        assert!(plan.omitted.contains(&"d".to_string()) && !plan.omitted.contains(&"c".to_string()), "{:?}", plan.omitted);
        assert!(plan.omitted.iter().filter(|u| u.starts_with("c/")).count() >= 1, "{:?}", plan.omitted);
        assert!(plan.omitted.last().unwrap() == "d" && plan.omitted.iter().all(|u| u == "d" || u.as_str() > "c/f00.txt"));
        // Omitted files are never read: unreadable, they would otherwise be problems.
        let omitted = observed.plan_frame(&shape(true), without(&["c", "d"])).omitted;
        for i in 0..10 {
            for dir in ["c", "d"] {
                fs::set_permissions(tree.0.join(format!("{dir}/f{i:02}.txt")), fs::Permissions::from_mode(0)).unwrap();
            }
        }
        observed.omit(&omitted);
        let wanted: BTreeSet<String> = wanted.into_iter().filter(|p| !within(&omitted, p)).collect();
        let mut read = Vec::new();
        session.stream(&listing, &mut observed, &wanted, |_, path, bytes| {
            if !bytes.is_empty() {
                read.push(path.to_string());
            }
            Ok(())
        }).unwrap();
        assert!(observed.problems.is_empty(), "{:?}", observed.problems);
        assert_eq!(read.len(), 21);
        assert!(read.iter().all(|p| !within(&omitted, p)));
        let issues: Vec<&str> = observed.issues.iter().filter(|i| i.1 == OMITTED).map(|i| i.0.as_str()).collect();
        assert_eq!(issues, ["c", "d"]);
        assert!(!observed.complete);
    }

    #[test]
    fn frame_estimate_matches_the_inline_frame() {
        let tree = fixture();
        let session = tree.session(limits(1000, MAX, MAX));
        let listing = session.enumerate().unwrap();
        let wanted: BTreeSet<String> = Observation::listed(&listing).unwrap().files.into_keys().filter(|p| p.starts_with('a') || p.starts_with('R')).collect();
        let captured = session.capture(&listing, Some(&wanted)).unwrap();
        let empty = BTreeSet::new();
        let shape = FrameShape { wanted: &wanted, native: &empty, kept: &empty, chunked: false, fixed: 0, partials: 0 };
        let estimate = captured.plan_frame(&shape, usize::MAX).frame;
        let actual = captured.frame(&wanted).len();
        assert!(estimate.abs_diff(actual) * 100 <= actual * 2, "{estimate} vs {actual}");
    }
}
