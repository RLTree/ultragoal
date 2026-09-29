use super::*;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;

/// Target size of one ranking chunk frame and of one source-window plan frame. The
/// core takes about 0.8 s and 100 MB per 2 MiB SELECT frame and evaluates up to
/// `FRAMES_PER_REQUEST` frames of one request in parallel; a request carries at most
/// `REQUEST_BYTES` of frames unless one frame alone is larger.
const CHUNK_BYTES: usize = 2 * 1024 * 1024;
const FRAMES_PER_REQUEST: usize = 8;
const REQUEST_BYTES: usize = 16 * 1024 * 1024;
/// Source bytes held while their window plan is pending.
const HELD_BYTES: usize = 8 * 1024 * 1024;
/// Default shortlist size G (`--shortlist N`); Bend validates it and pools the first 4G files.
const SHORTLIST: &str = "128";

/// Test and operator override of the chunk frame size; the frame bound stays MAX.
fn chunk_budget() -> Result<usize, String> {
    let adaptive = resources::current().map(|h| h.frame_bytes(MAX)).unwrap_or(CHUNK_BYTES);
    if adaptive == 0 { return Err("selection paused by current memory pressure; retry after headroom recovers".into()); }
    match env::var("UG_SELECT_CHUNK_BYTES") {
        Err(_) => Ok(CHUNK_BYTES.min(adaptive)),
        Ok(v) => v
            .parse::<usize>()
            .ok()
            .filter(|n| (1024..=MAX).contains(n))
            .ok_or_else(|| format!("UG_SELECT_CHUNK_BYTES must be 1024..={MAX}")),
    }
}

/// Names in `scope`, in input order; Bend scopes each row alone, so frames are cut
/// at the chunk size.
fn scoped(names: impl Iterator<Item = String>, scope: &str) -> Result<Vec<String>, String> {
    let head = "PLAN\nUG\t1\n".to_string() + &row(&["Q", scope]);
    let mut names = names.peekable();
    let mut out = Vec::new();
    loop {
        let mut frame = head.clone();
        while frame.len() < CHUNK_BYTES
            && let Some(name) = names.next()
        {
            frame += &row(&["F", &name, "", ""]);
        }
        for r in evaluate(frame)? {
            if r.len() == 2 && r[0] == "READ" {
                out.push(r[1].clone());
            } else {
                return Err("invalid scope query result".into());
            }
        }
        if names.peek().is_none() {
            return Ok(out);
        }
    }
}

/// One excerpt offered to Bend as a `C` row with id `c{index}`. Excerpts are
/// interned, so byte-identical excerpts share one allocation.
struct Candidate {
    path: Rc<str>,
    digest: Rc<str>,
    start: usize,
    end: usize,
    excerpt: Rc<str>,
}
impl Candidate {
    fn row(&self, i: usize) -> String {
        row(&["C", &format!("c{i}"), &self.path, &self.digest, &self.start.to_string(), &self.end.to_string(), &self.excerpt])
    }
    /// `self.row(i).len()` without building it.
    fn row_len(&self, i: usize) -> usize {
        let digits = |n: usize| n.to_string().len();
        8 + 1 + digits(i) + enc_len(&self.path) + enc_len(&self.digest) + digits(self.start) + digits(self.end) + enc_len(&self.excerpt)
    }
}
fn candidate_index(id: &str, count: usize) -> Result<usize, String> {
    id.strip_prefix('c')
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|i| *i < count && format!("c{i}") == id)
        .ok_or_else(|| "unknown Bend candidate reference".to_string())
}

enum Outcome {
    Changed,
    Anchor(String, String),
    Source { digest: Rc<str>, mismatched: usize, windows: Vec<(bool, usize, usize, Option<Rc<str>>)> },
}

/// The fact spans of one set with a source line, checked against current bytes later. A
/// span is kept in 32 bits: captured files are at most `MAX` bytes, so a larger offset
/// never matches and is counted as `bad`.
#[derive(Default)]
struct Spans {
    /// Spans whose facts carry no value: valid when they bound UTF-8 in the file.
    plain: Vec<(u32, u32)>,
    /// Spans whose facts carry their text, which must equal the file's bytes there.
    inline: Vec<(u32, u32, Box<str>)>,
    /// Facts that can never match (a value where none is allowed, or none where one is).
    bad: usize,
}

impl Spans {
    /// Takes one fact `[kind, value, line, start, end]`; `line` 0 is metadata.
    fn fact(&mut self, fields: &[Value], compact: bool, mixed: bool) -> Result<(), String> {
        let number = |k: usize, missing: &'static str, invalid: &'static str| -> Result<usize, String> {
            fields[k].as_str().ok_or(missing)?.parse::<usize>().map_err(|_| invalid.to_string())
        };
        if number(2, "line", "line number")? == 0 {
            return Ok(());
        }
        let start = number(3, "start", "start number")?;
        let end = number(4, "end", "end number")?;
        self.span(start, end, &fields[1], compact, mixed);
        Ok(())
    }

    fn span(&mut self, start: usize, end: usize, value: &Value, compact: bool, mixed: bool) {
        let (Ok(a), Ok(b)) = (u32::try_from(start), u32::try_from(end)) else {
            self.bad += 1;
            return;
        };
        if compact && !value.is_null() {
            self.bad += 1;
        } else if compact || (mixed && value.is_null()) {
            self.plain.push((a, b));
        } else if let Some(text) = value.as_str() {
            self.inline.push((a, b, text.into()));
        } else {
            self.bad += 1;
        }
    }

    /// One set's facts: `facts` in Bend's five fields, then `line_runs`, whose lines are
    /// `line` facts without a value (see `fact_set` in main.rs).
    fn of(set: &Value) -> Result<Self, String> {
        let compact = set["representation"] == "source-spans/2";
        let mixed = set["representation"] == "mixed-spans/1";
        let mut spans = Self::default();
        for fact in set["facts"].as_array().ok_or("facts array")? {
            let fields = fact.as_array().ok_or("fact record")?;
            if fields.len() != 5 {
                return Err("fact record shape".into());
            }
            spans.fact(fields, compact, mixed)?;
        }
        let Some(runs) = set.get("line_runs") else { return Ok(spans) };
        for run in runs.as_array().ok_or("line runs array")? {
            let numbers = run.as_array().filter(|r| r.len() >= 3).ok_or("line run shape")?;
            let numbers = numbers.iter().map(|n| n.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or("line run number")).collect::<Result<Vec<_>, _>>()?;
            let (mut line, mut start) = (numbers[0], numbers[1]);
            for &length in &numbers[2..] {
                let end = start.checked_add(length).ok_or("line run number")?;
                if line != 0 {
                    spans.span(start, end, &Value::Null, compact, mixed);
                }
                line = line.checked_add(1).ok_or("line run number")?;
                start = end.checked_add(1).ok_or("line run number")?;
            }
        }
        Ok(spans)
    }

    /// Valid spans against the file's current bytes, sorted and deduplicated, and how
    /// many facts do not match them.
    fn check(&self, bytes: &[u8]) -> (Vec<(usize, usize)>, usize) {
        let (mut lines, mut mismatched) = (Vec::new(), self.bad);
        let fits = |a: usize, b: usize| a <= b && b <= bytes.len() && std::str::from_utf8(&bytes[a..b]).is_ok();
        for &(a, b) in &self.plain {
            let (a, b) = (a as usize, b as usize);
            if fits(a, b) { lines.push((a, b)) } else { mismatched += 1 }
        }
        for (a, b, text) in &self.inline {
            let (a, b) = (*a as usize, *b as usize);
            if fits(a, b) && &bytes[a..b] == text.as_bytes() { lines.push((a, b)) } else { mismatched += 1 }
        }
        lines.sort_unstable();
        lines.dedup();
        (lines, mismatched)
    }
}

/// What `select` keeps of one fact set. A set's fact errors surface only when its spans
/// are needed, as when the whole report was decoded.
struct FactSet {
    path: Option<String>,
    digest: Option<String>,
    excluded: bool,
    spans: Result<Spans, String>,
}

/// The fields of an index report that `select` reads.
struct Report {
    root: Value,
    scope: Value,
    manifest: Value,
    obligations: Value,
    sets: Option<Vec<FactSet>>,
}

/// Decodes a report one value at a time: each fact set is reduced to a `FactSet` as it is
/// read, so memory follows the spans kept rather than the report's size. Keys are unique
/// at every level, as `provider::decode` requires.
impl<'de> serde::Deserialize<'de> for Report {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
        struct Sets;
        impl<'de> DeserializeSeed<'de> for Sets {
            type Value = Option<Vec<FactSet>>;
            fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                d.deserialize_any(self)
            }
        }
        impl<'de> Visitor<'de> for Sets {
            type Value = Option<Vec<FactSet>>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("index fact sets")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut items: A) -> Result<Self::Value, A::Error> {
                let mut sets = Vec::new();
                while let Some(set) = items.next_element::<provider::Strict>()? {
                    let set = set.0;
                    sets.push(FactSet {
                        path: set["path"].as_str().map(str::to_string),
                        digest: set["content_sha256"].as_str().map(str::to_string),
                        excluded: set["grammar"] == "private-source-excluded",
                        spans: Spans::of(&set),
                    });
                }
                Ok(Some(sets))
            }
            fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }
        }
        struct Fields;
        impl<'de> Visitor<'de> for Fields {
            type Value = Report;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("an index report object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut fields: A) -> Result<Report, A::Error> {
                let mut report = Report { root: Value::Null, scope: Value::Null, manifest: Value::Null, obligations: Value::Null, sets: None };
                let mut seen = HashSet::new();
                while let Some(key) = fields.next_key::<String>()? {
                    if !seen.insert(key.clone()) {
                        return Err(A::Error::custom("duplicate key"));
                    }
                    match key.as_str() {
                        "fact_sets" => report.sets = fields.next_value_seed(Sets)?,
                        name => {
                            let value = fields.next_value::<provider::Strict>()?.0;
                            match name {
                                "root" => report.root = value,
                                "scope" => report.scope = value,
                                "manifest" => report.manifest = value,
                                "obligations" => report.obligations = value,
                                _ => {}
                            }
                        }
                    }
                }
                Ok(report)
            }
        }
        d.deserialize_map(Fields)
    }
}

/// Feeds every byte read to a digest.
struct Hashing<R>(R, Sha256, u64);
impl<R: Read> Read for Hashing<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let n = self.0.read(buffer)?;
        self.1.update(&buffer[..n]);
        self.2 += n as u64;
        Ok(n)
    }
}

/// Reads a report as a stream, with its sha256; like `read`, it follows no link and
/// refuses a file that changes while it is read.
fn read_report(path: &Path) -> Result<(Report, String), String> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))?;
    let before = file.metadata().map_err(|e| e.to_string())?;
    if !before.is_file() {
        return Err("nonregular or oversized input".into());
    }
    let mut reader = Hashing(file, Sha256::new(), 0);
    let mut decoder = serde_json::Deserializer::from_reader(std::io::BufReader::with_capacity(1 << 20, &mut reader));
    let report = serde::Deserialize::deserialize(&mut decoder).map_err(|e| e.to_string())?;
    decoder.end().map_err(|e| e.to_string())?;
    drop(decoder);
    let after = reader.0.metadata().map_err(|e| e.to_string())?;
    if reader.2 != after.len()
        || before.ino() != after.ino()
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
    {
        return Err("input changed during read".into());
    }
    Ok((report, format!("{:x}", reader.1.finalize())))
}

/// Source files waiting for their Bend window plan, with their bytes. The plan runs
/// when its frame or the held bytes reach their bound, then excerpts are cut and the
/// bytes dropped, so memory follows the excerpts rather than the repository.
struct Planner {
    frame: String,
    held: Vec<(String, Rc<str>, Vec<u8>, Vec<(usize, usize)>, usize)>,
    bytes: usize,
    done: HashMap<String, Outcome>,
    pool: HashSet<Rc<str>>,
}
impl Planner {
    fn push(&mut self, path: &str, digest: String, bytes: Vec<u8>, spans: Vec<(usize, usize)>, mismatched: usize) -> Result<(), String> {
        let mut fields = vec![path.to_string()];
        for (start, end) in &spans {
            fields.push(start.to_string());
            fields.push(end.to_string());
        }
        self.frame += &row(&fields.iter().map(String::as_str).collect::<Vec<_>>());
        self.bytes += bytes.len();
        self.held.push((path.to_string(), digest.into(), bytes, spans, mismatched));
        if self.frame.len() >= CHUNK_BYTES || self.bytes >= HELD_BYTES {
            self.plan()?;
        }
        Ok(())
    }
    fn plan(&mut self) -> Result<(), String> {
        if self.held.is_empty() {
            return Ok(());
        }
        let planned = evaluate(std::mem::replace(&mut self.frame, "WINDOW_PLAN\n".into()))?;
        let slot: HashMap<&str, usize> = self.held.iter().enumerate().map(|(k, h)| (h.0.as_str(), k)).collect();
        let mut windows = vec![Vec::new(); self.held.len()];
        let mut seen = BTreeSet::new();
        for r in planned {
            if r.len() != 4 || !matches!(r[0].as_str(), "WINDOW" | "OVERSIZE") {
                return Err("Bend refused source-window plan".into());
            }
            let k = *slot.get(r[1].as_str()).ok_or("unrequested source-window path")?;
            let (_, _, bytes, spans, _) = &self.held[k];
            let start = r[2].parse::<usize>().map_err(|_| "window start")?;
            let end = r[3].parse::<usize>().map_err(|_| "window end")?;
            if start > end
                || end > bytes.len()
                || !seen.insert((k, start, end, r[0].clone()))
                || !spans.iter().any(|(a, _)| *a == start)
                || !spans.iter().any(|(_, b)| *b == end)
            {
                return Err("window does not bind captured source spans".into());
            }
            if r[0] == "OVERSIZE" {
                if !spans.contains(&(start, end)) {
                    return Err("oversize result does not bind a source span".into());
                }
                windows[k].push((true, start, end, None));
            } else {
                let excerpt = std::str::from_utf8(&bytes[start..end]).map_err(|_| "excerpt UTF-8")?;
                let interned = match self.pool.get(excerpt) {
                    Some(e) => e.clone(),
                    None => {
                        let e: Rc<str> = excerpt.into();
                        self.pool.insert(e.clone());
                        e
                    }
                };
                windows[k].push((false, start, end, Some(interned)));
            }
        }
        for ((path, digest, _, _, mismatched), windows) in self.held.drain(..).zip(windows) {
            self.done.insert(path, Outcome::Source { digest, mismatched, windows });
        }
        self.bytes = 0;
        Ok(())
    }
}

/// Consecutive rows of `set` cut into chunks of about `budget` bytes, at a file boundary
/// when a whole file fits; a file larger than the budget is cut inside it, since every
/// ranking stage sums the partial rows of one file.
fn chunks(set: &[usize], candidates: &[Candidate], budget: usize) -> Vec<Vec<usize>> {
    let (mut out, mut cur, mut bytes, mut k) = (Vec::new(), Vec::new(), 0, 0);
    while k < set.len() {
        let path = &candidates[set[k]].path;
        let end = set[k..].iter().position(|&i| !Rc::ptr_eq(&candidates[i].path, path)).map_or(set.len(), |n| k + n);
        let size: usize = set[k..end].iter().map(|&i| candidates[i].row_len(i)).sum();
        if !cur.is_empty() && bytes + size > budget {
            out.push(std::mem::take(&mut cur));
            bytes = 0;
        }
        for &i in &set[k..end] {
            let cost = candidates[i].row_len(i);
            if !cur.is_empty() && bytes + cost > budget {
                out.push(std::mem::take(&mut cur));
                bytes = 0;
            }
            cur.push(i);
            bytes += cost;
        }
        k = end;
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn frame(head: &str, set: &[usize], candidates: &[Candidate]) -> String {
    let mut frame = head.to_string();
    for &i in set {
        frame += &candidates[i].row(i);
    }
    frame
}

fn encoded(r: &[String]) -> String {
    row(&r.iter().map(String::as_str).collect::<Vec<_>>())
}

/// Core requests of the ranking stages: one frame through `evaluate()`, or several chunk
/// frames per request on a dedicated lane, which evaluates the frames of one request in parallel.
struct Stages {
    lane: Option<core_session::Lane>,
    frames: usize,
    requests: usize,
}
impl Stages {
    /// Every answer row, in frame order; a Bend ERROR row fails the selection.
    fn run(&mut self, frames: Vec<String>) -> Result<Vec<Vec<String>>, String> {
        self.frames += frames.len();
        let mut rows = Vec::new();
        if frames.len() == 1 {
            self.requests += 1;
            rows = evaluate(frames.into_iter().next().unwrap_or_default())?;
        } else {
            let mut pending = frames.into_iter().peekable();
            while pending.peek().is_some() {
                let (mut batch, mut bytes) = (Vec::new(), 0);
                while let Some(f) = pending.next_if(|f| batch.is_empty() || (batch.len() < FRAMES_PER_REQUEST && bytes + f.len() <= REQUEST_BYTES)) {
                    bytes += f.len();
                    batch.push(f);
                }
                self.requests += 1;
                let lane = match &mut self.lane {
                    Some(lane) => lane,
                    empty => empty.insert(core_session::Lane::new("select")?),
                };
                rows.extend(decode_core_rows(lane.request(batch, MAX)?)?);
            }
        }
        match rows.iter().find(|r| r.first().is_some_and(|t| t == "ERROR")) {
            Some(r) => Err(format!("Bend refused select ranking: {}", r.get(1).map_or("", String::as_str))),
            None => Ok(rows),
        }
    }

    /// Keep the global corpus together in Bend while transporting its exact rows
    /// in bounded, ordered units. Each ACK precedes the next page (backpressure).
    fn rank_paged(&mut self, query: &str, shortlist: &str, partial: &[Vec<String>], excluded: &[String]) -> Result<Vec<Vec<String>>, String> {
        let operation = format!("select-{}", std::process::id());
        let total = partial.len().checked_add(excluded.len()).ok_or("ranking row count overflow")?;
        let lane = match &mut self.lane {
            Some(lane) => lane,
            empty => empty.insert(core_session::Lane::new("select-rank")?),
        };
        let mut command = |fields: &[&str]| -> Result<Vec<Vec<String>>, String> {
            self.frames += 1;
            self.requests += 1;
            decode_core_rows(lane.request_one(row(fields), MAX)?)
        };
        let ack = command(&["RANK_BEGIN", &operation, query, shortlist, &total.to_string()])?;
        if ack != vec![vec!["RANK_ACK".to_string(), "begin".to_string()]] { return Err("Bend ranking begin refused".into()); }
        let test_limit = env::var("UG_SELECT_RANK_PAGE_BYTES").ok().and_then(|v| v.parse::<usize>().ok());
        let mut sequence = 0usize;
        let mut page = String::new();
        let mut limit = resources::current().map(|h| h.frame_bytes(MAX)).unwrap_or(CHUNK_BYTES);
        if limit == 0 { return Err("global rank paused by current memory pressure".into()); }
        limit = limit.min(test_limit.unwrap_or(MAX)).min(MAX).max(1024);
        for r in partial.iter().map(|r| encoded(r)).chain(excluded.iter().map(|p| row(&["X", p]))) {
            // Percent encoding the page as a field can triple its wire size.
            if !page.is_empty() && row(&["RANK_PAGE", &operation, &sequence.to_string(), &(page.clone() + &r)]).len() > limit {
                let ack = command(&["RANK_PAGE", &operation, &sequence.to_string(), &page])?;
                if ack != vec![vec!["RANK_ACK".to_string(), "page".to_string()]] { return Err("Bend ranking page refused".into()); }
                sequence += 1;
                page.clear();
                limit = resources::current().map(|h| h.frame_bytes(MAX)).unwrap_or(CHUNK_BYTES);
                if limit == 0 { return Err("global rank paused by current memory pressure".into()); }
                limit = limit.min(test_limit.unwrap_or(MAX)).min(MAX).max(1024);
            }
            page += &r;
            if row(&["RANK_PAGE", &operation, &sequence.to_string(), &page]).len() > MAX {
                let _ = command(&["RANK_ABORT"]);
                return Err("one ranking row exceeds the core's physical frame bound".into());
            }
        }
        if !page.is_empty() {
            let ack = command(&["RANK_PAGE", &operation, &sequence.to_string(), &page])?;
            if ack != vec![vec!["RANK_ACK".to_string(), "page".to_string()]] { return Err("Bend ranking page refused".into()); }
            sequence += 1;
        }
        let mut reply = command(&["RANK_END", &operation, &sequence.to_string()])?;
        let (mut assembled, mut offset) = (String::new(), 0usize);
        loop {
            let [data] = reply.as_slice() else { return Err("Bend ranking output page shape".into()); };
            if data.len() != 5 || data[0] != "RANK_DATA" { return Err(format!("Bend ranking output refused: {data:?}")); }
            let start = usize::try_from(number(&data[1])?).map_err(|_| "ranking output offset overflow")?;
            let next = usize::try_from(number(&data[2])?).map_err(|_| "ranking output offset overflow")?;
            if start != offset || Some(next) != start.checked_add(data[4].chars().count()) || !matches!(data[3].as_str(), "more" | "done") {
                return Err("Bend ranking output offset or marker mismatch".into());
            }
            assembled.push_str(&data[4]);
            offset = next;
            if data[3] == "done" { break; }
            if resources::current().is_some_and(|h| assembled.len() as u64 > h.work_bytes() / 8) {
                let _ = command(&["RANK_ABORT"]);
                return Err("ranking output paused by current memory pressure; retry from revalidated index".into());
            }
            reply = command(&["RANK_READ", &operation, &offset.to_string()])?;
        }
        let rows = decode_core_rows(assembled.into_bytes())?;
        if rows.iter().any(|r| r.first().is_some_and(|t| t == "RANK_ERROR" || t == "ERROR")) { return Err("Bend ranking output refused".into()); }
        Ok(rows)
    }
}

struct Ranked {
    rows: Vec<Vec<String>>,
    coverage: Value,
    excluded: Vec<String>,
    measurements: Value,
}

fn tagged<'a>(rows: &'a [Vec<String>], tag: &str, fields: usize) -> Result<Vec<&'a Vec<String>>, String> {
    let out: Vec<_> = rows.iter().filter(|r| r[0] == tag).collect();
    if out.iter().any(|r| r.len() != fields) {
        return Err(format!("invalid Bend {tag} row"));
    }
    Ok(out)
}

fn number(s: &str) -> Result<u64, String> {
    s.parse::<u64>().map_err(|_| "invalid Bend ranking number".to_string())
}

/// Bend's four ranking stages over all candidates (Retrieval.bend). Stage 1 counts terms
/// per file and per chunk of whole files; stage 2 sums those rows, which are exact partial
/// sums, and ranks every file by BM25F into a pool of 4G; stage 3 finds each pooled file's
/// best window; stage 4 fuses the two ranks and keeps G excerpt groups. Rust only packs
/// frames, concatenates answers and binds ids; any grouping of rows into chunks gives the
/// same answer. Byte-identical rows of a chosen window past the frame bound are left out
/// of its aliases and named.
fn rank(query: &str, shortlist: &str, candidates: &[Candidate], excluded: &[String]) -> Result<Ranked, String> {
    let started = Instant::now();
    let budget = chunk_budget()?;
    let mut st = Stages { lane: None, frames: 0, requests: 0 };
    let all: Vec<usize> = (0..candidates.len()).collect();

    let t = Instant::now();
    let match_head = "SELECT_MATCH\n".to_string() + &row(&["QUERY", query]);
    let frames: Vec<String> = chunks(&all, candidates, budget.min(MAX - match_head.len())).iter().map(|c| frame(&match_head, c, candidates)).collect();
    let (match_frames, match_bytes) = (frames.len(), frames.iter().map(String::len).sum::<usize>());
    let partial = if frames.is_empty() { Vec::new() } else { st.run(frames)? };
    if partial.iter().any(|r| r[0] != "FILE" && r[0] != "WINDOWS") {
        return Err("invalid Bend term count row".into());
    }
    let match_ms = t.elapsed().as_millis();

    let t = Instant::now();
    let rank_head = "FILE_RANK\n".to_string() + &row(&["QUERY", query]) + &row(&["SHORTLIST", shortlist]);
    let rank_bytes = rank_head.len() + partial.iter().map(|r| encoded(r).len()).sum::<usize>()
        + excluded.iter().map(|p| row(&["X", p]).len()).sum::<usize>();
    if resources::current().is_some_and(|h| rank_bytes as u64 > h.work_bytes() / 16) {
        return Err("global rank paused by current memory pressure; retry from the revalidated index".into());
    }
    let ranked = if rank_bytes <= MAX && env::var_os("UG_SELECT_RANK_PAGE_BYTES").is_none() {
        let mut rank_frame = rank_head;
        for r in &partial { rank_frame += &encoded(r); }
        for p in excluded { rank_frame += &row(&["X", p]); }
        st.run(vec![rank_frame])?
    } else { st.rank_paged(query, shortlist, &partial, excluded)? };
    let head = tagged(&ranked, "RANKED", 5)?;
    let [head] = head.as_slice() else { return Err("Bend ranking summary missing".into()) };
    let pool = tagged(&ranked, "POOL", 4)?;
    let idf = tagged(&ranked, "IDF", 4)?;
    let norm = tagged(&ranked, "NORM", 3)?;
    let [norm] = norm.as_slice() else { return Err("Bend window norm missing".into()) };
    let matching: Vec<String> = tagged(&ranked, "EXCLUDED", 2)?.iter().map(|r| r[1].clone()).collect();
    let rank_ms = t.elapsed().as_millis();

    let t = Instant::now();
    let pooled: HashSet<&str> = pool.iter().map(|r| r[1].as_str()).collect();
    let members: Vec<usize> = all.iter().copied().filter(|&i| pooled.contains(&*candidates[i].path)).collect();
    let mut passage_head = "SELECT_PASSAGE\n".to_string() + &row(&["QUERY", query]) + &encoded(norm);
    for r in &idf {
        passage_head += &encoded(r);
    }
    let frames: Vec<String> = chunks(&members, candidates, budget.min(MAX - passage_head.len())).iter().map(|c| frame(&passage_head, c, candidates)).collect();
    let (passage_frames, passage_bytes) = (frames.len(), frames.iter().map(String::len).sum::<usize>());
    let bests = if frames.is_empty() { Vec::new() } else { st.run(frames)? };
    if bests.iter().any(|r| r.len() != 4 || r[0] != "BEST") {
        return Err("invalid Bend best-window row".into());
    }
    let passage_ms = t.elapsed().as_millis();

    // Fuse only the best windows. Other byte-identical candidates cannot be
    // representatives; Bend compares all of them in separate alias pages.
    let t = Instant::now();
    let mut fuse = "SELECT_FUSE\n".to_string() + &row(&["SHORTLIST", shortlist]);
    for r in pool.iter().copied().chain(&bests) {
        fuse += &encoded(r);
    }
    let best: BTreeSet<usize> = bests.iter().map(|r| candidate_index(&r[2], candidates.len())).collect::<Result<_, _>>()?;
    let rows: Vec<usize> = best.iter().copied().collect();
    let fuse = frame(&fuse, &rows, candidates);
    let fuse_bytes = fuse.len();
    let mut answer = st.run(vec![fuse])?;
    let stop = tagged(&answer, "STOP", 2)?;
    let [stop] = stop.as_slice() else { return Err("Bend shortlist stop missing".into()) };
    let stopping_score = number(&stop[1])?;
    answer.retain(|r| r[0] != "STOP");
    let fuse_ms = t.elapsed().as_millis();

    let t = Instant::now();
    let primary: Vec<String> = answer.iter().filter(|r| r[0] == "CANDIDATE")
        .map(|r| row(&["PRIMARY", &r[1], &r[3], &r[4], &r[5], &r[6], &r[7]])).collect();
    let others: Vec<usize> = all.iter().copied().filter(|i| !best.contains(i)).collect();
    let (mut alias_frames, mut alias_bytes) = (0usize, 0usize);
    let mut at = 0;
    while at < primary.len() && !others.is_empty() {
        let mut head = "SELECT_ALIASES\n".to_string();
        while at < primary.len() && (head.len() == "SELECT_ALIASES\n".len() || head.len() + primary[at].len() <= MAX / 2) {
            head += &primary[at];
            at += 1;
        }
        let room = MAX.checked_sub(head.len()).ok_or("selected alias reference exceeds physical core frame")?;
        let frames: Vec<String> = chunks(&others, candidates, budget.min(room)).iter().map(|c| frame(&head, c, candidates)).collect();
        alias_frames += frames.len();
        alias_bytes += frames.iter().map(String::len).sum::<usize>();
        let aliases = st.run(frames)?;
        if aliases.iter().any(|r| r.len() != 7 || r[0] != "ALIAS") { return Err("invalid Bend alias page".into()); }
        answer.extend(aliases);
    }
    let alias_ms = t.elapsed().as_millis();

    let cores = st.lane.take().map(|lane| lane.finish()).transpose()?.unwrap_or(0);
    let coverage = json!({"shortlist":number(&head[3])?,"pool":number(&head[4])?,"files":number(&head[1])?,"files_matched":number(&head[2])?,"stopping_score":stopping_score});
    let measurements = json!({"route":if match_frames > 1 {"chunked"} else {"single-frame"},"chunk_budget":budget,"frame_limit":MAX,
        "stages":{"match":{"frames":match_frames,"bytes":match_bytes,"rows":partial.len(),"ms":match_ms},
                  "file_rank":{"bytes":rank_bytes,"pooled_files":pool.len(),"ms":rank_ms},
                  "passage":{"frames":passage_frames,"bytes":passage_bytes,"candidates":members.len(),"ms":passage_ms},
                  "fuse":{"bytes":fuse_bytes,"rows":rows.len(),"ms":fuse_ms},
                  "aliases":{"frames":alias_frames,"bytes":alias_bytes,"ms":alias_ms}},
        "core_frames":st.frames,"core_requests":st.requests,"core_processes":cores,"unranked":0,"ms":started.elapsed().as_millis()});
    Ok(Ranked { rows: answer, coverage, excluded: matching, measurements })
}

/// Streams `[items]` as serde_json would serialize the whole array into a digest.
struct HashWriter(Sha256);
impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn select(args: &[String]) -> Result<(Value, i32), String> {
    let started = Instant::now();
    let query = option(args, "--query")?.ok_or("--query required")?;
    if query.trim().is_empty() || query.len() > 2000 {
        return Err("query must contain 1..2000 bytes".into());
    }
    let shortlist = option(args, "--shortlist")?.unwrap_or_else(|| SHORTLIST.into());
    let report_path = option(args, "--report")?.ok_or("--report index/check output required")?;
    let (report, report_digest) = read_report(Path::new(&report_path))?;
    let root = fs::canonicalize(option(args, "--root")?.ok_or("--root required")?)
        .map_err(|e| e.to_string())?;
    if report.root.as_str() != root.to_str() {
        return Err("index root does not match requested root".into());
    }
    let scope = report.scope.as_str().unwrap_or("*").to_string();
    let old_names = report.manifest
        .as_array()
        .ok_or("index lacks membership manifest; refresh it")?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or("manifest path".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let sets = report.sets.as_ref().ok_or("index lacks fact sets")?;
    // Files the index excluded as private source: Bend names those whose path matches the
    // question, by path only; nothing of them reaches a provider.
    let excluded: Vec<String> = sets.iter().filter(|s| s.excluded).filter_map(|s| s.path.clone()).collect();
    let mut by_path = HashMap::new();
    for (k, set) in sets.iter().enumerate() {
        let path = set.path.as_deref().ok_or("fact path")?;
        let relative = Path::new(path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("unsafe fact path".into());
        }
        if by_path.insert(path, k).is_some() {
            return Err("duplicate fact set path".into());
        }
    }
    let root_instruction = |p: &str| p == "AGENTS.md" || p.ends_with("/AGENTS.md") || p == "GOAL_CONTRACT.md";
    let binding_source = |p: &str| Path::new(p).file_name().is_some_and(|x| x == "AGENTS.md" || x == "GOAL_CONTRACT.md");
    let observer = inventory::Session::open(&root)?;
    let baseline = observer.baseline()?;
    let listing = baseline.listing();
    let mut observed = observer.capture(listing, None)?;
    let mut wanted: BTreeSet<String> = by_path.keys().map(|p| p.to_string()).collect();
    wanted.extend(observed.kinds.keys().filter(|n| root_instruction(n)).cloned());
    let old_members = scoped(old_names.into_iter(), &scope)?;
    let current_members = scoped(observed.kinds.keys().cloned(), &scope)?;
    let (old_set, current_set): (HashSet<&String>, HashSet<&String>) = (old_members.iter().collect(), current_members.iter().collect());
    let added: Vec<_> = current_members.iter().filter(|p| !old_set.contains(p)).cloned().collect();
    let removed: Vec<_> = old_members.iter().filter(|p| !current_set.contains(p)).cloned().collect();
    // Each wanted file's facts are checked and its excerpts cut while its bytes are
    // in hand; only excerpts and root instruction texts are kept.
    let mut planner = Planner { frame: "WINDOW_PLAN\n".into(), held: Vec::new(), bytes: 0, done: HashMap::new(), pool: HashSet::new() };
    let mut instructions = BTreeMap::new();
    observer.stream(listing, &mut observed, &wanted, |observed, path, bytes| {
        if observed.problems.contains_key(path) || !wanted.contains(path) {
            return Ok(());
        }
        if root_instruction(path) {
            instructions.insert(path.to_string(), (hash(&bytes), std::str::from_utf8(&bytes).map_err(|_| "anchor UTF-8")?.to_string()));
        }
        let Some(&k) = by_path.get(path) else { return Ok(()) };
        let digest = hash(&bytes);
        if sets[k].digest.as_deref() != Some(digest.as_str()) {
            planner.done.insert(path.to_string(), Outcome::Changed);
        } else if binding_source(path) {
            let text = std::str::from_utf8(&bytes).map_err(|_| "anchor UTF-8")?.to_string();
            planner.done.insert(path.to_string(), Outcome::Anchor(digest, text));
        } else {
            let (lines, mismatched) = sets[k].spans.as_ref().map_err(String::clone)?.check(&bytes);
            planner.push(path, digest, bytes, lines, mismatched)?;
        }
        Ok(())
    })?;
    planner.plan()?;
    let mut outcomes = planner.done;
    let mut candidates = Vec::new();
    let mut changed = Vec::new();
    let mut unsupported = Vec::new();
    let mut anchors = Vec::new();
    if let Some(obligations) = report.obligations.as_array() {
        for o in obligations {
            anchors.push(json!({"kind":"requirement-retained-unranked","id":o["id"],"statement":o["requirement"],"origin":o["origin"]}));
        }
    }
    let mut sources = Vec::new();
    for set in sets {
        let path = set.path.as_deref().ok_or("fact path")?;
        if !observed.kinds.contains_key(path) {
            changed.push(path.to_string());
        } else if let Some(problem) = observed.problems.get(path) {
            unsupported.push(json!({"path":path,"problem":problem}));
        } else {
            match outcomes.remove(path) {
                None | Some(Outcome::Changed) => changed.push(path.to_string()),
                Some(Outcome::Anchor(digest, text)) => {
                    anchors.push(json!({"kind":"binding-source-retained-unranked","path":path,"sha256":digest,"text":text}));
                }
                Some(Outcome::Source { digest, mismatched, windows }) => {
                    for _ in 0..mismatched {
                        unsupported.push(json!({"path":path,"reason":"fact span does not match current bytes"}));
                    }
                    sources.push((path, digest, windows));
                }
            }
        }
    }
    for (path, digest, windows) in sources {
        let shared: Rc<str> = path.into();
        for (oversize, start, end, excerpt) in windows {
            match excerpt {
                Some(excerpt) if !oversize => candidates.push(Candidate { path: shared.clone(), digest: digest.clone(), start, end, excerpt }),
                _ => unsupported.push(json!({"path":path,"start":start,"end":end,"reason":"source line exceeds bounded excerpt; adjacent facts remain available"})),
            }
        }
    }
    // The report's spans are the largest allocation; nothing below reads them.
    drop(report);
    // Root instructions remain visible even when the requested index scope excluded them.
    for path in wanted.iter().filter(|p| root_instruction(p)) {
        if !anchors.iter().any(|a| a["path"] == *path) {
            if let Some((digest, text)) = instructions.get(path) {
                anchors.push(json!({"kind":"binding-source-retained-unranked","path":path,"sha256":digest,"text":text}));
            } else {
                anchors.push(json!({"kind":"binding-source-unavailable","path":path,"problem":observed.problems.get(path)}));
            }
        }
    }
    let Ranked { rows: ranked, coverage: ranking_coverage, excluded: excluded_matching, measurements: ranking } = rank(&query, &shortlist, &candidates, &excluded)?;
    let mut shortlist = Vec::new();
    let mut alias_rows = Vec::new();
    let mut alias_limit = None;
    let mut rubric = None;
    for r in ranked {
        if r.first().is_some_and(|s| s == "PROVENANCE_LIMIT") {
            if r.len() != 2 || alias_limit.is_some() { return Err("invalid provenance limit record".into()); }
            let limit = r[1].parse::<usize>().map_err(|_| "provenance limit")?;
            if limit > 128 { return Err("provenance transport count bound".into()); }
            alias_limit = Some(limit);
            continue;
        }
        if r.first().is_some_and(|s| s == "RUBRIC") {
            if r.len() != 9 {
                return Err("relevance rubric shape".into());
            }
            rubric = Some(r);
            continue;
        }
        if r.first().is_some_and(|s| s == "ALIAS") {
            if r.len() != 7 { return Err("invalid Bend alias envelope".into()); }
            alias_rows.push(r);
            continue;
        }
        if r.len() != 8 || r[0] != "CANDIDATE" {
            return Err("invalid Bend selection result".into());
        }
        shortlist.push(json!({"id":r[1],"lexical_score":r[2].parse::<u32>().map_err(|_|"lexical score")?,"path":r[3],"input_sha256":r[4],"start":r[5].parse::<usize>().map_err(|_|"start")?,"end":r[6].parse::<usize>().map_err(|_|"end")?,"excerpt":r[7],"aliases":[]}));
    }
    // Bend chooses groups. The physical adapter binds each returned reference
    // to the already captured candidate metadata; it does not rerank evidence.
    let mut represented = BTreeMap::<String, String>::new();
    for c in &shortlist {
        let id = c["id"].as_str().ok_or("candidate identity")?.to_string();
        if represented.insert(id.clone(), id).is_some() {
            return Err("duplicate Bend representative".into());
        }
    }
    for r in alias_rows {
        let original = &candidates[candidate_index(&r[2], candidates.len()).map_err(|_| "unknown Bend alias reference")?];
        let start = r[5].parse::<usize>().map_err(|_| "alias start")?;
        let end = r[6].parse::<usize>().map_err(|_| "alias end")?;
        if *original.path != r[3] || *original.digest != r[4] || original.start != start || original.end != end {
            return Err("Bend alias reference differs from captured metadata".into());
        }
        let primary = shortlist.iter_mut().find(|c| c["id"] == r[1])
            .ok_or("unknown Bend alias representative")?;
        if represented.insert(r[2].clone(), r[1].clone()).is_some() {
            return Err("duplicate or conflicting Bend alias reference".into());
        }
        primary["aliases"].as_array_mut().ok_or("alias list")?.push(
            json!({"id":r[2],"path":r[3],"input_sha256":r[4],"start":start,"end":end}));
    }
    // Pages can arrive after best-window aliases; restore the original candidate
    // order after Bend has decided exact group membership.
    for c in &mut shortlist {
        c["aliases"].as_array_mut().ok_or("alias list")?.sort_by_key(|a| {
            a["id"].as_str().and_then(|id| id.strip_prefix('c'))
                .and_then(|number| number.parse::<usize>().ok()).unwrap_or(usize::MAX)
        });
    }
    let alias_limit = alias_limit.ok_or("Bend provenance limit missing")?;
    let local_ms = started.elapsed().as_millis();
    let rubric = rubric.ok_or("Bend relevance rubric missing")?;
    let prepared_evidence_digest = hash(&serde_json::to_vec(&shortlist).map_err(|e| e.to_string())?);
    let mut criteria = serde_json::Map::new();
    for pair in rubric[3..].chunks_exact(2) {
        criteria.insert(pair[0].clone(), json!(pair[1]));
    }
    // Shortlist order is Bend's ranking, which picks the window's head.
    let items = shortlist
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let qid = format!("item{i}");
            let aliases = c["aliases"].as_array().ok_or("alias metadata")?;
            let shown_aliases = aliases.iter().take(alias_limit).map(|a| &a["path"]).collect::<Vec<_>>();
            Ok(provider::WindowItem {
                state: json!({"path":c["path"],"start":c["start"],"end":c["end"],"excerpt":c["excerpt"],"aliases":shown_aliases,"unshown_aliases":aliases.len().saturating_sub(alias_limit)}),
                question: json!({"type":"choice","instructions":rubric[2].replace("{candidate}",&format!("state.candidates.{qid}")),"criteria":criteria}),
                id: qid,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let fixed = json!({"schema":"ultragoal-relevance-input/3","rubric":rubric[1],"query":query,"candidates":{},"provenance_scope":"Each candidate shows one excerpt and bounded duplicate paths; unshown aliases are not individually assessed."});
    let request_digest = hash(&serde_json::to_vec(&json!({"fixed":fixed,"items":items.iter().map(|i|json!([i.id,i.state,i.question])).collect::<Vec<_>>()})).map_err(|e| e.to_string())?);
    let prior = option(args, "--prior")?
        .map(|p| read(Path::new(&p), MAX).and_then(|b| provider::decode(&b)))
        .transpose()?;
    let prior_same = prior
        .as_ref()
        .is_some_and(|p| p["request_sha256"] == request_digest && p["prepared_evidence_sha256"] == prepared_evidence_digest);
    // Prior model output never suppresses a live required check or becomes exact evidence.
    let mut window = None;
    let mut window_head = None;
    let assessment = if prior_same && !args.contains(&"--semantic".to_string()) {
        Some(
            json!({"state":"reported-prior","reported":prior.as_ref().map(|p|p["assessment"].clone()),"attempts":[],"reason":"same question and byte-bound evidence; prior advice retained without treating its mutable provenance as a current assessment"}),
        )
    } else if !shortlist.is_empty() && option(args, "--disclosure")?.is_some() {
        let outcome = provider::windowed(args, &fixed, &items, &provider::budget(args)?)?;
        let a = json!({"state":if outcome.judgments.values().all(|j|j.is_ok()){"advisory"}else{"advisory-partial"},"window":outcome.plan,"requests":outcome.assessments});
        window_head = Some(outcome.head_request);
        window = Some(outcome.judgments);
        Some(a)
    } else {
        None
    };
    // Bend's fitted policy decides relevance from each validated answer's probabilities.
    let mut rows = String::new();
    for (id, judged) in window.iter().flatten() {
        if let Ok((a, _)) = judged {
            let p = |name: &str| a["answer"]["probabilities"][name].as_number().map(|n| n.to_string()).unwrap_or_default();
            rows += &row(&[id, &p("relevant"), &p("irrelevant"), &p("insufficient")]);
        }
    }
    let decided = if rows.is_empty() { Vec::new() } else { evaluate("RELEVANCE_DECISIONS\n".to_string() + &rows)? };
    let decisions: BTreeMap<String, (String, String)> = decided
        .into_iter()
        .map(|r| match r.as_slice() {
            [tag, id, decision, policy] if tag == "RELEVANCE" => Ok((id.clone(), (decision.clone(), policy.clone()))),
            _ => Err("Bend relevance decision shape".to_string()),
        })
        .collect::<Result<_, String>>()?;
    for (i, c) in shortlist.iter_mut().enumerate() {
        c["assessment_provenance"] = json!({"shown_aliases":c["aliases"].as_array().unwrap().len().min(alias_limit),"unshown_aliases":c["aliases"].as_array().unwrap().len().saturating_sub(alias_limit),"scope":"displayed excerpt and displayed provenance only; unshown aliases remain unassessed"});
        let judged = window.as_ref().and_then(|w| w.get(&format!("item{i}")));
        let decision = decisions.get(&format!("item{i}"));
        c["relevance"] = json!(decision.map_or("unassessed", |(d, _)| d.as_str()));
        if let (Some((_, policy)), Some(Ok((a, _)))) = (decision, judged) {
            c["relevance_policy"] = json!(policy);
            c["model_choice"] = a["label"].clone();
        }
        if let Some(Err(reason)) = judged {
            c["uninspected_reason"] = json!(reason);
        }
        if let Some(Ok((_, request))) = judged {
            c["window_request"] = json!(request);
            c["comparison_scope"] = json!(window_head.map(|head| provider::comparison_scope(head, *request)));
        }
    }
    let (final_observation, revalidation_mode) = observer.revalidate(&baseline, &wanted)?;
    let changed_after: Vec<String> = observed
        .changes(&final_observation)
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    for c in &mut shortlist {
        c["current"] = json!(
            !changed_after
                .iter()
                .any(|p| p.is_empty() || c["path"] == *p
                    || c["aliases"].as_array().is_some_and(|aliases|
                        aliases.iter().any(|a| a["path"] == *p)))
                && final_observation.complete
        );
        if c["current"] == false {
            c["relevance"] = json!("stale");
        }
    }
    let selected_bytes: usize = shortlist
        .iter()
        .filter(|c| c["relevance"] != "irrelevant")
        .map(|c| c["excerpt"].as_str().unwrap().len())
        .sum();
    let mut summary = "SELECT_STATUS\n".to_string();
    for c in &shortlist {
        summary += c["relevance"].as_str().ok_or("relevance")?;
        summary.push('\n');
    }
    let selection_status = evaluate(summary)?;
    // The provenance digest covers every candidate; the list is built only on request.
    let details = args.contains(&"--details".to_string());
    let mut provenance = Vec::new();
    let mut digest = HashWriter(Sha256::new());
    digest.write_all(b"[").map_err(|e| e.to_string())?;
    for (i, c) in candidates.iter().enumerate() {
        let id = format!("c{i}");
        let item = json!({"id":id,"path":&*c.path,"input_sha256":&*c.digest,"start":c.start,"end":c.end,"in_shortlist":represented.contains_key(&id),"represented_by":represented.get(&id)});
        if i > 0 {
            digest.write_all(b",").map_err(|e| e.to_string())?;
        }
        serde_json::to_writer(&mut digest, &item).map_err(|e| e.to_string())?;
        if details {
            provenance.push(item);
        }
    }
    digest.write_all(b"]").map_err(|e| e.to_string())?;
    let provenance_digest = format!("{:x}", digest.0.finalize());
    let unassessed: Vec<Value> = Vec::new();
    for c in &mut shortlist {
        if c["relevance"] == "irrelevant" {
            c.as_object_mut().unwrap().remove("excerpt");
        }
    }
    let retained_anchor_bytes: usize = anchors
        .iter()
        .map(|a| {
            a["text"]
                .as_str()
                .or_else(|| a["statement"].as_str())
                .unwrap_or("")
                .len()
        })
        .sum();
    Ok((
        json!({"schema":"ultragoal-context/1","state":"advisory","selection_status":selection_status,"query":query,"root":root,"scope":scope,"request_sha256":request_digest,"prepared_evidence_sha256":prepared_evidence_digest,"rubric":rubric[1],"candidate_count":candidates.len(),"candidate_provenance":if details{Some(&provenance)}else{None},"provenance_sha256":provenance_digest,"recovery":{"index_path":report_path,"index_sha256":report_digest,"instruction":"Use --details to recover the complete candidate metadata; exact source spans stay in the index and current source."},"shortlist":shortlist,"anchors":anchors,"coverage":{"omitted_from_shortlist":candidates.len().saturating_sub(represented.len()),"represented_candidates":represented.len(),"returned_groups":shortlist.len(),"added_paths":added,"removed_paths":removed,"changed_paths":changed,"changed_after_assessment":changed_after,"unsupported":unsupported,"unassessed":unassessed,"ranking":ranking_coverage,"excluded_matching":excluded_matching,"revalidation_mode":revalidation_mode,"listing_complete":observed.complete,"listing_issues":observed.issues,"critical_evidence_recall":"unknown outside independently labelled evaluation","completeness":"reported index revalidated by exact spans; no global-completeness or mandatory-pruning claim"},"reuse":{"same_prior_question_and_prepared_evidence":prior_same,"source_facts_reparsed":0,"prior_assessment_used_as_verified_evidence":false,"action":"reuse unchanged source excerpts; refresh only changed/new dependencies; prior model output remains reported advisory"},"assessment":assessment,"measurements":{"local_discovery_revalidation_ranking_ms":local_ms,"total_ms":started.elapsed().as_millis(),"returned_excerpt_bytes":selected_bytes,"duplicate_candidates_represented_as_aliases":represented.len().saturating_sub(shortlist.len()),"retained_anchor_bytes":retained_anchor_bytes,"primary_agent_input_tokens":null,"false_omission_recovery_ms":null,"ranking":ranking},"next_action":"Read relevant and insufficient excerpts with retained anchors; inspect omitted/new/changed evidence before claiming an answer is complete.","assurance":"optional advisory context selection; never filters binding instructions or mandatory checks"}),
        0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_length_matches_the_encoded_row() {
        let c = Candidate { path: "a b/%\t.rs".into(), digest: "ab".repeat(32).into(), start: 7, end: 1234, excerpt: "fn x() {\r\n\t100%\n}".into() };
        for i in [0, 9, 10, 123456] {
            assert_eq!(c.row_len(i), c.row(i).len());
        }
    }

    /// The report of `fact_set` and the earlier form (every fact in `facts`) give `select`
    /// the same spans and mismatches, for every representation.
    #[test]
    fn line_runs_give_the_spans_of_line_facts() {
        let facts = [
            ["line", "", "1", "0", "3"], ["line", "", "2", "4", "4"], ["line", "", "3", "5", "9"],
            ["definition-candidate", "def f", "4", "10", "15"], ["line", "", "5", "21", "25"],
            ["line", "", "6", "27", "30"], ["line", "", "7", "31", "29"], ["line", "", "08", "31", "32"],
            ["line", "", "9", "33", "99"], ["json-valid", "", "0", "0", "0"], ["line", "x", "10", "0", "1"],
        ];
        let body: String = facts.iter().map(|f| row(f)).collect();
        for tag in ["P_SPANS", "P_MIXED", "P"] {
            let r = [tag, "a.py", "d", "g", &body, "parsed", &"a".repeat(64)].map(String::from);
            let set = fact_set(&r).unwrap();
            if tag == "P_SPANS" {
                assert_eq!(set["line_runs"], json!([[1, 0, 3, 0, 4], [5, 21, 4], [6, 27, 3], [9, 33, 66], [10, 0, 1]]));
                assert_eq!(set["facts"].as_array().unwrap().len(), 4);
            }
            let (compact, mixed) = (tag == "P_SPANS", tag == "P_MIXED");
            let mut legacy = set.clone();
            legacy.as_object_mut().unwrap().remove("line_runs");
            legacy["facts"] = facts
                .iter()
                .map(|f| {
                    let null = (compact || (mixed && f[1].is_empty() && f[3] != f[4])) && f[2] != "0";
                    json!([f[0], if null { Value::Null } else { json!(f[1]) }, f[2], f[3], f[4]])
                })
                .collect();
            let (new, old) = (Spans::of(&set).unwrap(), Spans::of(&legacy).unwrap());
            for bytes in ["a".repeat(40), "é".repeat(20), "def f".repeat(3), String::new()] {
                assert_eq!(new.check(bytes.as_bytes()), old.check(bytes.as_bytes()), "{tag}");
            }
        }
    }

    #[test]
    fn provenance_writer_hashes_the_same_json_bytes_without_external_output() {
        let item = json!({"id":"c0","path":"src/a.py","start":3,"end":9});
        let mut sink = HashWriter(Sha256::new());
        serde_json::to_writer(&mut sink, &item).unwrap();
        assert_eq!(format!("{:x}", sink.0.finalize()),
                   hash(&serde_json::to_vec(&item).unwrap()));
    }

    #[test]
    fn report_keys_are_unique_at_every_level() {
        let decode = |text: &str| serde_json::from_str::<Report>(text).map(|r| r.sets.map_or(0, |s| s.len()));
        assert_eq!(decode(r#"{"root":"/r","fact_sets":[{"path":"a"}]}"#).unwrap(), 1);
        assert!(decode(r#"{"root":"/r","root":"/s"}"#).is_err());
        assert!(decode(r#"{"fact_sets":[{"path":"a","path":"b"}]}"#).is_err());
        assert!(decode(r#"{"inputs":{"a":1,"a":2}}"#).is_err());
    }

    #[test]
    fn chunks_cut_at_file_boundaries_and_inside_larger_files() {
        let (a, b, c): (Rc<str>, Rc<str>, Rc<str>) = ("a".into(), "b".into(), "c".into());
        let excerpt: Rc<str> = "x".repeat(20).into();
        let row = |path: &Rc<str>| Candidate { path: path.clone(), digest: "d".into(), start: 0, end: 20, excerpt: excerpt.clone() };
        let cands = vec![row(&a), row(&a), row(&b), row(&b), row(&b), row(&c)];
        let one = cands[0].row_len(0);
        // Whole files while they fit; a file larger than the budget is cut inside it.
        assert_eq!(chunks(&[0, 1, 2, 3, 4, 5], &cands, 2 * one + 2), [vec![0, 1], vec![2, 3], vec![4, 5]]);
        assert_eq!(chunks(&[0, 1, 2, 3, 4, 5], &cands, 4 * one), [vec![0, 1], vec![2, 3, 4, 5]]);
        assert_eq!(chunks(&[2, 5], &cands, 100 * one), [vec![2, 5]]);
    }
}
