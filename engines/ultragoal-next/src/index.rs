//! Streamed index transport; Bend still selects scope and parses every source.
use super::*;
use std::collections::BTreeSet;
use std::sync::mpsc;

/// Listing bytes per index request (named risk: machine RAM). A request's eight frames
/// are one Bend batch; its core holds about 50 bytes per request byte on the 10x fixture
/// and 80 on real text, so two lanes at 16 MiB peak at 1.8 and 2.6 GB (32 MiB: 3.4, 4.6).
pub const REQUEST_BYTES: u64 = 16 * 1024 * 1024;
const FRAMES: usize = 8;

fn encode_group(header: &str, group: &[(&str, &[u8])]) -> Result<String, String> {
    let mut counts = std::collections::HashMap::<&[u8], usize>::new();
    for (_, bytes) in group {
        *counts.entry(bytes).or_default() += 1;
    }
    let intern = counts.values().any(|n| *n > 1);
    let mut frame = format!(
        "{}\n{header}",
        if intern { "INDEX_INTERNED" } else { "INDEX" }
    );
    let mut atoms = std::collections::HashMap::<&[u8], (String, String)>::new();
    for (path, bytes) in group {
        let source = std::str::from_utf8(bytes).map_err(|_| "index UTF-8")?;
        if counts.get(bytes).copied().unwrap_or(0) == 1 {
            frame += &row(&["F", path, &hash(bytes), source]);
        } else {
            let (id, digest) = match atoms.get(bytes) {
                Some(value) => value.clone(),
                None => {
                    let id = atoms.len().to_string();
                    let digest = hash(bytes);
                    frame += &row(&["D", &id, &digest, source]);
                    atoms.insert(bytes, (id.clone(), digest.clone()));
                    (id, digest)
                }
            };
            frame += &row(&["F_REF", path, &digest, &id]);
        }
        if frame.len() > MAX {
            return Err(format!("index input frame exceeds16MiB for {path}"));
        }
    }
    Ok(frame)
}

fn expanded(path: &str, bytes: &[u8]) -> usize {
    enc_len(std::str::from_utf8(bytes).unwrap_or("")) + path.len() * 3 + 128
}

/// One request's files in frames of about equal expanded size: the core evaluates
/// them together and a batch lasts about as long as its largest frame.
fn frames(header: &str, files: &[(String, Vec<u8>)]) -> Result<Vec<String>, String> {
    let sizes: Vec<usize> = files.iter().map(|(p, b)| expanded(p, b)).collect();
    let target = sizes.iter().sum::<usize>().div_ceil(FRAMES).max(1);
    let room = MAX - header.len() - 32;
    let mut frames = Vec::new();
    let mut group: Vec<(&str, &[u8])> = Vec::new();
    let (mut placed, mut held) = (0usize, 0usize);
    for ((path, bytes), size) in files.iter().zip(sizes) {
        if !group.is_empty() && (placed + size > target * (frames.len() + 1) || held + size > room) {
            frames.push(encode_group(header, &group)?);
            (group, held) = (Vec::new(), 0);
        }
        placed += size;
        held += size;
        group.push((path, bytes));
    }
    if !group.is_empty() || frames.is_empty() {
        frames.push(encode_group(header, &group)?);
    }
    Ok(frames)
}

/// Files captured for the request being filled.
struct Pending {
    files: Vec<(String, Vec<u8>)>,
    bytes: usize,
    placed: u64,
    room: usize,
}

impl Pending {
    /// Takes a wanted file unless it has a problem or its row cannot fit a frame,
    /// which is named as a problem instead of failing the index.
    fn admit(&mut self, observed: &mut inventory::Observation, wanted: &BTreeSet<String>, path: &str, content: Vec<u8>) -> bool {
        if !wanted.contains(path) || observed.problems.contains_key(path) {
            return false;
        }
        if 7 + enc_len(path) + 64 + enc_len(std::str::from_utf8(&content).unwrap_or("")) > self.room {
            observed.problems.insert(path.into(), ("file".into(), "exceeds index frame bound".into()));
            return false;
        }
        self.bytes += content.len();
        self.placed += observed.identities.get(path).map_or(0, |i| i.size);
        self.files.push((path.to_string(), content));
        true
    }

    fn take(&mut self, header: &str) -> Result<(Vec<String>, String), String> {
        let files = std::mem::take(&mut self.files);
        let span = format!("{} .. {}", files.first().map_or("", |f| f.0.as_str()), files.last().map_or("", |f| f.0.as_str()));
        Ok((frames(header, &files)?, span))
    }
}

type Answer = (usize, Result<Vec<u8>, String>);
/// Takes each core answer in request order; nothing keeps an answer after it returns.
pub type Sink<'a> = &'a mut dyn FnMut(Vec<u8>) -> Result<(), String>;

fn flush_pending(pending: &mut Pending, header: &str, sink: &mut Sink<'_>) -> Result<(), String> {
    if pending.files.is_empty() { return Ok(()) }
    let (frames, span) = pending.take(header)?;
    let answer = core_session::request(frames, true, Instant::now() + core_session::REQUEST_STALL, REPORT_MAX)
        .map_err(|e| format!("index request for {span}: {e}"))?;
    sink(answer)?;
    pending.placed = 0;
    Ok(())
}

/// Requests alternate over two lanes; answers go to the sink in request order.
struct Dispatch<'a> {
    lanes: Vec<mpsc::SyncSender<(usize, Vec<String>, String)>>,
    answers: mpsc::Receiver<Answer>,
    sent: usize,
    pending: BTreeMap<usize, Vec<u8>>,
    next: usize,
    sink: Sink<'a>,
}

impl Dispatch<'_> {
    fn take(&mut self, (i, answer): Answer) -> Result<(), String> {
        self.pending.insert(i, answer?);
        while let Some(answer) = self.pending.remove(&self.next) {
            (self.sink)(answer)?;
            self.next += 1;
        }
        Ok(())
    }

    fn send(&mut self, (frames, span): (Vec<String>, String)) -> Result<(), String> {
        let concurrency = crate::resources::current().map_or(1, |h| h.index_lanes());
        if self.lanes[self.sent % concurrency].send((self.sent, frames, span)).is_err() {
            self.lanes.clear();
            while let Ok(answer) = self.answers.recv() {
                self.take(answer)?;
            }
            return Err("index lane stopped".into());
        }
        self.sent += 1;
        while let Ok(answer) = self.answers.try_recv() {
            self.take(answer)?;
        }
        Ok(())
    }

    fn finish(mut self) -> Result<(), String> {
        self.lanes.clear();
        while let Ok(answer) = self.answers.recv() {
            self.take(answer)?;
        }
        if self.next != self.sent {
            return Err("incomplete index response".into());
        }
        Ok(())
    }
}

/// Serves requests in order, each on a fresh core: a core slows as its allocator ages
/// (IMPLEMENTATION_STATUS, "Growth over long sessions"; at 10x 445 s of CPU with one core
/// per lane, 211 s with one per request), and its memory is freed between requests.
fn lane(requests: mpsc::Receiver<(usize, Vec<String>, String)>, done: mpsc::Sender<Answer>) {
    for (i, frames, span) in requests {
        let answer = core_session::Lane::new("index")
            .and_then(|mut core| {
                let answer = core.request(frames, REPORT_MAX)?;
                core.finish()?;
                Ok(answer)
            })
            .map_err(|e| format!("index request {i}: {}; paths {span}", e.strip_prefix("index request 0: ").unwrap_or(&e)));
        let failed = answer.is_err();
        let _ = done.send((i, answer));
        if failed {
            return;
        }
    }
}

/// Captures and indexes `wanted` one request at a time: requests of about 16 MiB
/// alternate over two local lanes, and each answer goes to `sink` in request order as
/// it arrives. The owner core of a session takes an index only when it is one request,
/// since the owner's cumulative byte bound also serves later checks. Returns the bytes
/// indexed.
pub fn evaluate_index(
    observer: &inventory::Session,
    listing: &fs_adapter::Listing,
    observed: &mut inventory::Observation,
    wanted: &BTreeSet<String>,
    contract: &str,
    operation: &str,
    mut sink: Sink,
) -> Result<usize, String> {
    if crate::resources::current().is_some_and(|h| h.work_bytes() == 0) {
        return Err("index paused by current memory pressure; retry after headroom recovers".into());
    }
    let mut header = format!("{contract}\n");
    header += &row(&["OP", operation, "running", &hash(contract.as_bytes()), "timely"]);
    for excluded in &observed.exclusions {
        header += &row(&["X", excluded]);
    }
    let mut pending = Pending { files: Vec::new(), bytes: 0, placed: 0, room: MAX - header.len() - "INDEX_INTERNED\n".len() };
    let total = chunks::selected_bytes(listing, wanted);
    let count = wanted.iter().filter(|p| observed.kinds.get(*p).is_some_and(|k| k == "file")).count();
    if wanted.iter().any(|path| listing.members.iter().any(|m| m.path == path.as_bytes() && m.identity.size > (MAX / 3) as u64)) && count > 1 {
        let mut budget = crate::resources::current().map_or(512 * 1024, |h| h.frame_bytes(MAX)) as u64;
        observer.stream(listing, observed, wanted, |observed, path, content| {
            if !wanted.contains(path) || observed.problems.contains_key(path) { return Ok(()) }
            let needs_pages = expanded(path, &content) > pending.room;
            if needs_pages {
                flush_pending(&mut pending, &header, &mut sink)?;
                let result = segmented::evaluate_one(observer, path, operation, &header, "", segmented::Mode::Index).map_err(|e| e.to_string())?;
                if observed.identities.get(path) != Some(&result.source.identity) {
                    return Err(format!("{path}: source changed since listing"));
                }
                pending.bytes = pending.bytes.saturating_add(usize::try_from(result.source.bytes).map_err(|_| "source size exceeds host address space")?);
                let mut output = Vec::new();
                for r in result.rows {
                    output.extend_from_slice(row(&r.iter().map(String::as_str).collect::<Vec<_>>()).as_bytes());
                }
                sink(output)?;
            } else if pending.admit(observed, wanted, path, content) && pending.placed >= budget {
                flush_pending(&mut pending, &header, &mut sink)?;
                budget = crate::resources::current().map_or(512 * 1024, |h| h.frame_bytes(MAX)) as u64;
            }
            Ok(())
        })?;
        flush_pending(&mut pending, &header, &mut sink)?;
        return Ok(pending.bytes);
    }
    if total <= REQUEST_BYTES && (session_transport::enabled() || count < 16) {
        observer.stream(listing, observed, wanted, |observed, path, content| {
            pending.admit(observed, wanted, path, content);
            Ok(())
        })?;
        let (frames, _) = pending.take(&header)?;
        let answer = core_session::request(frames, true, Instant::now() + core_session::REQUEST_STALL, REPORT_MAX)
            .map_err(|e| format!("index request 0: {e}"))?;
        sink(answer)?;
        return Ok(pending.bytes);
    }
    let budget = crate::resources::current().map_or(512 * 1024, |h| h.frame_bytes(MAX) as u64)
        .min(REQUEST_BYTES);
    let per = total.div_ceil(2 * total.div_ceil(2 * budget).max(1)).max(1);
    std::thread::scope(|scope| {
        let (done, answers) = mpsc::channel();
        let lanes = (0..2)
            .map(|_| {
                let (tx, rx) = mpsc::sync_channel(0);
                let done = done.clone();
                scope.spawn(move || lane(rx, done));
                tx
            })
            .collect();
        drop(done);
        let mut dispatch = Dispatch { lanes, answers, sent: 0, pending: BTreeMap::new(), next: 0, sink };
        observer.stream(listing, observed, wanted, |observed, path, content| {
            if pending.admit(observed, wanted, path, content) && pending.placed >= per * (dispatch.sent as u64 + 1) {
                dispatch.send(pending.take(&header)?)?;
            }
            Ok(())
        })?;
        if !pending.files.is_empty() || dispatch.sent == 0 {
            dispatch.send(pending.take(&header)?)?;
        }
        dispatch.finish()
    })?;
    Ok(pending.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_interning_only_encodes_repeated_bodies() {
        let plain = encode_group("UG\t1\n", &[("a.py", b"first"), ("b.py", b"second")]).unwrap();
        assert!(plain.starts_with("INDEX\n"));
        assert!(!plain.contains("F_REF\t"));
        let mixed = encode_group(
            "UG\t1\n",
            &[("a.py", b"same"), ("b.py", b"unique"), ("c.py", b"same")],
        )
        .unwrap();
        assert!(mixed.starts_with("INDEX_INTERNED\n"));
        assert_eq!(mixed.lines().filter(|l| l.starts_with("D\t")).count(), 1);
        assert_eq!(
            mixed.lines().filter(|l| l.starts_with("F_REF\t")).count(),
            2
        );
        assert_eq!(mixed.lines().filter(|l| l.starts_with("F\t")).count(), 1);
        assert!(mixed.contains("\tunique\n"));
    }

    #[test]
    fn request_frames_are_even_and_within_the_frame_bound() {
        let files: Vec<(String, Vec<u8>)> = (0..40).map(|i| (format!("f{i:02}.py"), vec![b'a' + (i % 26) as u8; 100_000 + i * 1000])).collect();
        let even = frames("UG\t1\n", &files).unwrap();
        assert_eq!(even.len(), FRAMES);
        let sizes: Vec<usize> = even.iter().map(String::len).collect();
        assert!(sizes.iter().max().unwrap() - sizes.iter().min().unwrap() < 250_000, "{sizes:?}");
        let big: Vec<(String, Vec<u8>)> = (0..3).map(|i| (format!("b{i}.py"), vec![b'b' + i as u8; 9 * 1024 * 1024])).collect();
        let big = frames("UG\t1\n", &big).unwrap();
        assert_eq!(big.len(), 3);
        assert!(big.iter().all(|f| f.len() <= MAX));
    }
}
