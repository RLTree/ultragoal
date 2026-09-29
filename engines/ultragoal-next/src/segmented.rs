//! One logical source over bounded Bend session pages; no ordinary F row repeats.
use super::*;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::os::unix::ffi::OsStrExt;

pub struct Evaluated {
    pub rows: Vec<Vec<String>>,
    pub source: fs_adapter::StreamedSource,
    pub cores: usize,
}

#[derive(Debug)]
pub enum EvalError {
    Source(fs_adapter::StreamError),
    SourceChanged(String),
    Other(String),
}
impl From<String> for EvalError { fn from(value: String) -> Self { Self::Other(value) } }
impl From<&str> for EvalError { fn from(value: &str) -> Self { Self::Other(value.into()) } }
impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Source(e) => write!(f, "logical source: {e:?}"), Self::SourceChanged(path) => write!(f, "{path}: logical source changed since listing"), Self::Other(e) => f.write_str(e) }
    }
}

#[derive(Clone, Copy)]
pub enum Mode { Check, Index, Chunk }

pub fn frame_halves(
    header: &str,
    observed: &inventory::Observation,
    wanted: &BTreeSet<String>,
    path: &str,
    index: bool,
) -> Result<(String, String), String> {
    if wanted.len() != 1 || !wanted.contains(path) {
        return Err("logical source needs paged multi-file aggregation".into());
    }
    let mut prefix = header.to_string();
    let mut suffix = String::new();
    if index {
        for exclusion in &observed.exclusions {
            prefix += &row(&["X", exclusion]);
        }
    } else {
        prefix += &observed.head();
        let mut after = false;
        for name in observed.files.keys() {
            if name == path { after = true; continue }
            let physical = row(&["F", name, "unread", ""]);
            if after { suffix += &physical } else { prefix += &physical }
        }
        suffix += &observed.tail();
    }
    Ok((prefix, suffix))
}

fn ack(lane: &mut core_session::Lane, frame: String, phase: &str) -> Result<(), String> {
    let rows = decode_core_rows(lane.request_one(frame, 1024)?)?;
    if rows == [vec!["SEG_ACK".to_string(), phase.to_string()]] {
        Ok(())
    } else {
        Err(format!("Bend segment {phase} refused: {rows:?}"))
    }
}

/// A large advice request remains one exact source for Bend's whole-file
/// disclosure rule. The host retains the same descriptor bytes for JSON decode
/// only after Bend has screened the complete logical text and custody closes.
pub fn read_disclosed(path: &Path) -> Result<Vec<u8>, String> {
    let parent = path.parent().ok_or("advice request parent absent")?;
    let name = path.file_name().ok_or("advice request name absent")?.as_bytes();
    let canonical = path.to_str().ok_or("advice request path UTF-8")?;
    let root = fs_adapter::Root::open(parent).map_err(|e| format!("advice request root: {e}"))?;
    let operation = op_id()?;
    let lane = RefCell::new(core_session::Lane::new("advice-source")?);
    let version = RefCell::new(None::<String>);
    let bytes = RefCell::new(Vec::new());
    let sequence = Cell::new(0u64);
    let captured = root.stream_source(name, |meta| {
        let identity = hash(format!("{:?}", meta.identity).as_bytes());
        ack(&mut lane.borrow_mut(), row(&["SEG_BEGIN", &operation, canonical, &identity, &meta.digest, &meta.bytes.to_string()]), "begin")?;
        *version.borrow_mut() = Some(identity);
        Ok(())
    }, |offset, text| {
        let identity = version.borrow().clone().ok_or("advice segment begin absent")?;
        bytes.borrow_mut().try_reserve(text.len()).map_err(|_| "advice context needs more current headroom".to_string())?;
        bytes.borrow_mut().extend_from_slice(text.as_bytes());
        ack(&mut lane.borrow_mut(), row(&["SEG_PAGE", &operation, canonical, &identity, &sequence.get().to_string(), &offset.to_string(), text]), "page")?;
        sequence.set(sequence.get() + 1);
        Ok(())
    }).map_err(|e| format!("advice context source: {e:?}"))?;
    let identity = version.into_inner().ok_or("advice segment version absent")?;
    let mut lane = lane.into_inner();
    ack(&mut lane, row(&["SEG_END", &operation, canonical, &identity, &captured.digest, &sequence.get().to_string()]), "complete")?;
    let disclosure = decode_core_rows(lane.request_one(row(&["SEG_DISCLOSURE", &operation, canonical, &identity, &captured.digest]), 1024)?)?;
    lane.finish()?;
    if disclosure != vec![vec!["DISCLOSURE".to_string(), "eligible".to_string()]] {
        return Err("advice context excluded by disclosure policy".into());
    }
    let bytes = bytes.into_inner();
    if bytes.len() as u64 != captured.bytes || hash(&bytes) != captured.digest {
        return Err("advice context changed during screened capture".into());
    }
    Ok(bytes)
}

/// The C syntax admission rule sees one complete captured translation unit,
/// even when its physical request needs several core frames.
pub fn native_plan_captured(path: &Path, bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "source UTF-8")?;
    let path = path.to_str().ok_or("native source path UTF-8")?;
    let digest = hash(bytes);
    let version = hash(format!("{path}:{}:{digest}", bytes.len()).as_bytes());
    let operation = op_id()?;
    let mut lane = core_session::Lane::new("native source plan")?;
    ack(&mut lane, row(&["SEG_BEGIN", &operation, path, &version, &digest, &bytes.len().to_string()]), "begin")?;
    let (mut offset, mut sequence) = (0usize, 0usize);
    while offset < bytes.len() {
        if is_interrupted() { return Err("native source capture cancelled".into()); }
        let frame = resources::current().map(|h| h.frame_bytes(MAX)).unwrap_or(0);
        if frame == 0 { return Err(format!("native source plan paused at byte {offset} by current memory pressure")); }
        let mut end = offset.saturating_add((frame / 4).max(256)).min(bytes.len());
        while end > offset && !text.is_char_boundary(end) { end -= 1; }
        if end == offset { return Err("native source scalar exceeds physical page".into()); }
        ack(&mut lane, row(&["SEG_PAGE", &operation, path, &version, &sequence.to_string(), &offset.to_string(), &text[offset..end]]), "page")?;
        offset = end;
        sequence += 1;
    }
    ack(&mut lane, row(&["SEG_END", &operation, path, &version, &digest, &sequence.to_string()]), "complete")?;
    let result = decode_core_rows(lane.request_one(row(&["SEG_NATIVE_PLAN", &operation, path, &version, &digest]), 1024)?)?;
    lane.finish()?;
    Ok(result)
}

/// `prefix` and `suffix` are the ordinary sorted evaluation rows before and
/// after this file. Bend inserts the one F row after EOF and verifies that the
/// entire logical file digest matches the descriptor-bound source stream.
pub fn evaluate_one(
    observer: &inventory::Session,
    path: &str,
    operation: &str,
    prefix: &str,
    suffix: &str,
    mode: Mode,
) -> Result<Evaluated, EvalError> {
    let lane = RefCell::new(core_session::Lane::new("logical source")?);
    let version = RefCell::new(None::<String>);
    let seq = Cell::new(0u64);
    let total_bytes = Cell::new(0u64);
    let last_progress = Cell::new(0u64);
    let source = observer.stream_source(path, |meta| {
        total_bytes.set(meta.bytes);
        let identity = hash(format!("{:?}", meta.identity).as_bytes());
        ack(&mut lane.borrow_mut(), row(&["SEG_BEGIN", operation, path, &identity, &meta.digest, &meta.bytes.to_string()]), "begin")?;
        *version.borrow_mut() = Some(identity);
        Ok(())
    }, |offset, text| {
        let identity = version.borrow().clone().ok_or("segment begin absent")?;
        ack(&mut lane.borrow_mut(), row(&["SEG_PAGE", operation, path, &identity, &seq.get().to_string(), &offset.to_string(), text]), "page")?;
        seq.set(seq.get() + 1);
        let delivered = offset + text.len() as u64;
        if delivered.saturating_sub(last_progress.get()) >= 8 * 1024 * 1024 || delivered == total_bytes.get() {
            eprintln!("{}", json!({"event":"logical-source-progress","path":path,"delivered_bytes":delivered,"source_bytes":total_bytes.get()}));
            last_progress.set(delivered);
        }
        Ok(())
    }).map_err(EvalError::Source)?;
    let identity = version.into_inner().ok_or("segment version absent")?;
    let mut lane = lane.into_inner();
    ack(&mut lane, row(&["SEG_END", operation, path, &identity, &source.digest, &seq.get().to_string()]), "complete")?;
    let command = match mode { Mode::Check => "SEG_EVAL", Mode::Index => "SEG_INDEX", Mode::Chunk => "SEG_CHUNK" };
    let frame = row(&[command, operation, path, &identity, &source.digest, prefix, suffix]);
    if frame.len() > MAX {
        return Err(EvalError::Other("logical evaluation metadata needs operation paging".into()));
    }
    let rows = decode_core_rows(lane.request_one(frame, REPORT_MAX)?)?;
    if let Some(error) = rows.iter().find(|r| r.first().is_some_and(|x| x == "SEG_ERROR" || x == "ERROR")) {
        return Err(EvalError::Other(format!("Bend logical evaluation refused: {error:?}")));
    }
    let cores = lane.finish()?;
    Ok(Evaluated { rows, source, cores })
}

/// A mixed source set keeps path order and one CF row per logical file. This
/// slower path is used only when at least one selected file needs source pages;
/// the ordinary chunk batcher remains the common 10k-file route.
pub fn mixed_chunks(
    observer: &inventory::Session,
    listing: &fs_adapter::Listing,
    observed: &mut inventory::Observation,
    wanted: &BTreeSet<String>,
    contract: &str,
    operation: &str,
) -> Result<(chunks::Chunked, usize), EvalError> {
    mixed_chunks_with(observer, listing, observed, wanted, contract, operation, evaluate_one)
}

fn mixed_chunks_with(
    observer: &inventory::Session,
    listing: &fs_adapter::Listing,
    observed: &mut inventory::Observation,
    wanted: &BTreeSet<String>,
    contract: &str,
    operation: &str,
    mut evaluate_large: impl FnMut(&inventory::Session, &str, &str, &str, &str, Mode) -> Result<Evaluated, EvalError>,
) -> Result<(chunks::Chunked, usize), EvalError> {
    let started = Instant::now();
    let mut ordinary = core_session::Lane::new("mixed content chunk")?;
    let (mut output, mut count, mut total, mut cores) = (String::new(), 0usize, 0usize, 0usize);
    let paths: Vec<String> = observed.files.keys().cloned().collect();
    for path in paths {
        if is_interrupted() { return Err(EvalError::Source(fs_adapter::StreamError::Cancelled)) }
        let selected = wanted.contains(&path);
        let large = selected && listing.members.iter().any(|m| m.path == path.as_bytes() && m.identity.size > (MAX / 3) as u64);
        let index = count.to_string();
        let rows = if large {
            let prefix = format!("UG\t1\n{}", contract.strip_prefix("UG\t1\n").ok_or("contract header")?) + &row(&["CI", &index]);
            match evaluate_large(observer, &path, operation, &prefix, "", Mode::Chunk) {
                Ok(result) => {
                    if observed.identities.get(&path) != Some(&result.source.identity) {
                        return Err(EvalError::SourceChanged(path.clone()));
                    }
                    total = total.saturating_add(usize::try_from(result.source.bytes).map_err(|_| "source size exceeds host address space")?);
                    cores += result.cores;
                    result.rows
                }
                Err(EvalError::Source(fs_adapter::StreamError::Pressure { offset })) => {
                    // Partial pages for this source cannot be admitted. Bend sees
                    // it as unavailable while completed earlier CH rows survive.
                    observed.problems.insert(path.clone(), ("file".into(), format!("ResourcePressure at byte {offset}; retry from fresh source")));
                    let frame = format!("CHUNK\n{contract}\n") + &row(&["CI", &index]) + &row(&["F", &path, "unavailable", ""]);
                    decode_core_rows(ordinary.request(vec![frame], REPORT_MAX)?)?
                }
                Err(error) => return Err(error),
            }
        } else {
            let bytes = if selected {
                let one = BTreeSet::from([path.clone()]);
                let captured = observer.capture(listing, Some(&one))?;
                if let Some(problem) = captured.problems.get(&path) {
                    observed.problems.insert(path.clone(), problem.clone());
                }
                captured.files.get(&path).cloned().unwrap_or_default()
            } else { Vec::new() };
            if selected && !observed.problems.contains_key(&path) { total = total.saturating_add(bytes.len()); }
            let content = std::str::from_utf8(&bytes).unwrap_or("");
            let digest = observed.digest(&path, wanted, &bytes);
            let frame = format!("CHUNK\n{contract}\n") + &row(&["CI", &index]) + &row(&["F", &path, &digest, content]);
            decode_core_rows(ordinary.request(vec![frame], REPORT_MAX)?)?
        };
        if rows.iter().filter(|r| r.first().is_some_and(|x| x == "CH")).count() != 1 {
            return Err(EvalError::Other(format!("Bend mixed chunk {index} response incomplete")));
        }
        for r in &rows {
            output += &row(&r.iter().map(String::as_str).collect::<Vec<_>>());
        }
        count += 1;
    }
    cores += ordinary.finish()?;
    Ok((chunks::Chunked { rows: output, frames: count, requests: count, cores, ms: started.elapsed().as_millis() }, total))
}

#[cfg(test)]
mod pressure_tests {
    use super::*;
    use std::fs;

    #[test]
    fn completed_chunk_survives_later_pressure_and_mutated_source_is_relisted() {
        let root = std::fs::canonicalize(std::env::temp_dir()).unwrap().join(format!("ultragoal-pressure-{}", op_id().unwrap()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("a.txt"), b"kept earlier\n").unwrap();
        fs::write(root.join("b.txt"), vec![b'b'; MAX / 3 + 1024]).unwrap();
        let run = || -> Result<(), String> {
            let observer = inventory::Session::open(&root)?;
            let listing = observer.enumerate()?;
            let wanted = BTreeSet::from(["a.txt".into(), "b.txt".into()]);
            let mut observed = observer.identities(&listing, &wanted)?;
            let before = observed.identities["b.txt"];
            let contract = format!("UG\t1\n{}{}",
                row(&["O", "kept", "1", "Keep earlier text", "pressure regression", "a.txt", "content", "contains", "kept earlier", "exact", "mandatory"]),
                row(&["O", "later", "1", "Check later text", "pressure regression", "b.txt", "content", "contains", "recovered", "exact", "mandatory"])
                + &row(&["O", "across", "1", "Check all selected text", "pressure regression", "*", "content", "contains", "recovered", "exact", "mandatory"]));
            let (chunks, total) = mixed_chunks_with(&observer, &listing, &mut observed, &wanted, &contract, "pressure-test", |_observer, path, _operation, _prefix, _suffix, _mode| {
                assert_eq!(path, "b.txt");
                fs::write(root.join("b.txt"), vec![b'c'; MAX / 3 + 1024]).unwrap();
                Err(EvalError::Source(fs_adapter::StreamError::Pressure { offset: 4096 }))
            }).map_err(|e| e.to_string())?;
            assert_eq!(total, b"kept earlier\n".len());
            let chunk_states = chunks.rows.lines().filter(|line| line.starts_with("CH\t"))
                .map(|line| line.split('\t').nth(2).unwrap_or("").to_owned()).collect::<Vec<_>>();
            assert_eq!(chunk_states, ["complete", "complete"], "{}", chunks.rows);
            assert!(chunks.rows.contains("CF\tb.txt\tunavailable"), "{}", chunks.rows);
            assert!(chunks.rows.contains("a.txt"), "{}", chunks.rows);
            assert!(observed.problems["b.txt"].1.contains("byte 4096"));
            let frame = format!("{contract}\n") + &row(&["OP", "pressure-test", "running", "revision", "timely"])
                + &observed.head() + &row(&["FC"]) + &observed.tail() + &chunks.rows;
            let results = evaluate_check_frame(frame, "pressure-test")?;
            assert!(results.iter().any(|r| r.len() > 2 && r[0] == "RESULT" && r[1] == "kept" && r[2] == "verified"), "{results:?}");
            assert!(results.iter().any(|r| r.len() > 2 && r[0] == "RESULT" && r[1] == "later" && r[2] == "unknown"), "{results:?}");
            assert!(results.iter().any(|r| r.len() > 2 && r[0] == "RESULT" && r[1] == "across" && r[2] == "unknown"), "{results:?}");
            let fresh_listing = observer.enumerate()?;
            let fresh = observer.identities(&fresh_listing, &wanted)?;
            assert_ne!(fresh.identities["b.txt"], before);
            let mut cancelled = observer.identities(&fresh_listing, &wanted)?;
            let stopped = mixed_chunks_with(&observer, &fresh_listing, &mut cancelled, &wanted, &contract, "cancel-test", |_observer, _path, _operation, _prefix, _suffix, _mode| {
                Err(EvalError::Source(fs_adapter::StreamError::Cancelled))
            });
            assert!(matches!(stopped, Err(EvalError::Source(fs_adapter::StreamError::Cancelled))));
            fs::write(root.join("b.txt"), b"recovered\n").unwrap();
            let next_listing = observer.enumerate()?;
            let mut next = observer.identities(&next_listing, &wanted)?;
            let (resumed, total) = mixed_chunks(&observer, &next_listing, &mut next, &wanted, &contract, "next-use").map_err(|e| e.to_string())?;
            assert_eq!(total, b"kept earlier\n".len() + b"recovered\n".len());
            assert!(next.problems.is_empty(), "{:?}", next.problems);
            let frame = format!("{contract}\n") + &row(&["OP", "next-use", "running", "revision", "timely"])
                + &next.head() + &row(&["FC"]) + &next.tail() + &resumed.rows;
            let results = evaluate_check_frame(frame, "next-use")?;
            for id in ["kept", "later", "across"] {
                assert!(results.iter().any(|r| r.len() > 2 && r[0] == "RESULT" && r[1] == id && r[2] == "verified"), "{results:?}");
            }
            let earlier = results.iter().find(|r| r.first().is_some_and(|x| x == "RESULT") && r.get(1).is_some_and(|x| x == "kept")).unwrap();
            assert_eq!(earlier.get(7).map(String::as_str), Some("reused"), "{results:?}");
            let wildcard = results.iter().find(|r| r.first().is_some_and(|x| x == "RESULT") && r.get(1).is_some_and(|x| x == "across")).unwrap();
            assert_eq!(wildcard.get(7).map(String::as_str), Some("computed"), "{results:?}");
            Ok(())
        }();
        fs::remove_dir_all(&root).unwrap();
        run.unwrap();
    }
}
