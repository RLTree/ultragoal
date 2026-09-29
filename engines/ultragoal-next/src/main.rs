use serde_json::{Value, json};
use sha2::{Digest, Sha256};
mod core_session;
mod session_transport;
mod process;
use process::run;
pub mod fs_adapter;
mod inventory;
mod chunks;
mod native_identity;
mod native_syntax;
mod native_observation;
mod fit;
mod os;
mod relative;
mod resources;
mod segmented;
use std::{
    collections::BTreeMap,
    env,
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt},
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const MAX: usize = 16 * 1024 * 1024;
/// Largest core answer to one request (frontend memory per answer), and the output bound
/// of every report except `index`, whose fact sets are streamed.
const REPORT_MAX: usize = 256 * 1024 * 1024;
/// The `index` report while its fact sets are written: one set per line, as the core
/// answers them, so neither the answers nor the report are held. `main` closes the same
/// JSON object with the remaining fields, or with the failure.
static STREAMED: std::sync::Mutex<Option<Streamed>> = std::sync::Mutex::new(None);
struct Streamed {
    out: std::io::BufWriter<std::io::Stdout>,
    sets: usize,
    bytes: usize,
}
impl Streamed {
    fn open() -> Result<(), String> {
        let head = b"{\n  \"fact_sets\": [";
        let mut out = std::io::BufWriter::with_capacity(1 << 20, std::io::stdout());
        out.write_all(head).map_err(|e| format!("report output: {e}"))?;
        *STREAMED.lock().map_err(|_| "report stream")? = Some(Streamed { out, sets: 0, bytes: head.len() });
        Ok(())
    }
    fn write(set: &Value) -> Result<(), String> {
        let mut stream = STREAMED.lock().map_err(|_| "report stream")?;
        let s = stream.as_mut().ok_or("report stream closed")?;
        let line = serde_json::to_vec(set).map_err(|e| e.to_string())?;
        let separator: &[u8] = if s.sets == 0 { b"\n    " } else { b",\n    " };
        s.out.write_all(separator).and_then(|_| s.out.write_all(&line)).map_err(|e| format!("report output: {e}"))?;
        s.sets += 1;
        s.bytes += separator.len() + line.len();
        Ok(())
    }
    /// Ends the fact sets and appends the fields of `object`, a JSON object.
    fn close(mut self, object: &str) -> std::io::Result<usize> {
        let fields = object.strip_prefix('{').unwrap_or(object);
        self.out.write_all(b"\n  ],")?;
        self.out.write_all(fields.as_bytes())?;
        self.out.write_all(b"\n")?;
        self.out.flush()?;
        Ok(self.bytes + 5 + fields.len())
    }
}
/// Prints a report object, closing a streamed index report with it; returns stdout bytes.
fn emit(streamed: Option<Streamed>, object: &str) -> usize {
    let written = match streamed {
        None => {
            println!("{object}");
            Ok(object.len() + 1)
        }
        Some(s) => s.close(object),
    };
    written.unwrap_or_else(|e| {
        eprintln!("error: report output: {e}");
        std::process::exit(3)
    })
}
/// Line numbers and byte offsets exactly as Bend wrote them (no sign or leading zero).
fn canonical(fields: &[String]) -> Option<[u64; 3]> {
    let n = |s: &String| s.parse::<u64>().ok().filter(|n| n.to_string() == *s);
    Some([n(&fields[0])?, n(&fields[1])?, n(&fields[2])?])
}
/// One fact set as reports show it. Each fact keeps Bend's five fields (kind, value or
/// null, line, start, end), except that `line` facts without a value go to `line_runs`,
/// runs `[first line, start byte, length, length, ...]` in which each next line is
/// numbered one higher and starts one byte after the previous line's end. A line fact
/// that fits no run stays in `facts`, so every fact remains recoverable.
fn fact_set(r: &[String]) -> Result<Value, String> {
    if r.len() != 7 {
        return Err("invalid fact envelope".to_string());
    }
    let compact = r[0] == "P_SPANS";
    let mixed = r[0] == "P_MIXED";
    let (mut facts, mut runs) = (Vec::new(), Vec::<Vec<u64>>::new());
    let mut previous: Option<(u64, u64)> = None;
    for l in r[4].lines() {
        let fields = l.split('\t').map(dec).collect::<Result<Vec<_>, _>>()?;
        if fields.len() != 5 {
            return Err("fact record shape".to_string());
        }
        let span_only = compact || (mixed && fields[1].is_empty() && fields[3] != fields[4]);
        let null = span_only && fields[2] != "0";
        match (fields[0] == "line" && null).then(|| canonical(&fields[2..])).flatten() {
            Some([n, s, e]) if n > 0 && s <= e => {
                match (runs.last_mut(), previous) {
                    (Some(run), Some((pn, pe))) if pn.checked_add(1) == Some(n) && pe.checked_add(1) == Some(s) => run.push(e - s),
                    _ => runs.push(vec![n, s, e - s]),
                }
                previous = Some((n, e));
            }
            _ => facts.push(json!([fields[0], if null { Value::Null } else { json!(fields[1]) }, fields[2], fields[3], fields[4]])),
        }
    }
    Ok(json!({"path":r[1],"content_sha256":r[2],"grammar":r[3],"representation":if compact{"source-spans/2"}else if mixed{"mixed-spans/1"}else{"inline/1"},"facts":facts,"line_runs":runs,"computation":r[5],"key":digest_id(&r[6])?,"coverage":"declared bounded grammar and explicit limitations; source structure is not native syntax/type/runtime validity; metadata rows have no source spans"}))
}
static INTERRUPTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
extern "C" fn interrupted(_: libc::c_int) {
    INTERRUPTED.store(true, std::sync::atomic::Ordering::SeqCst);
}
fn is_interrupted() -> bool {
    INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest_id(value: &str) -> Result<String, String> {
    if value.len()!=64 || !value.bytes().all(|b|b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return Err("Bend computation identity is unavailable or malformed".into());
    }
    Ok(value.to_string())
}
fn enc(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\n', "%0A")
        .replace('\r', "%0D")
        .replace('\t', "%09")
}
/// Length of `enc(s)` without building it.
fn enc_len(s: &str) -> usize {
    s.len() + 2 * s.bytes().filter(|b| matches!(b, b'%' | b'\t' | b'\n' | b'\r')).count()
}
fn dec(s: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let x = chars.next().ok_or("bad escape")?;
        let y = chars.next().ok_or("bad escape")?;
        out.push(match (x, y) {
            ('2', '5') => '%',
            ('0', 'A') => '\n',
            ('0', 'D') => '\r',
            ('0', '9') => '\t',
            _ => return Err("bad escape".into()),
        });
    }
    Ok(out)
}
fn row(xs: &[&str]) -> String {
    xs.iter().map(|s| enc(s)).collect::<Vec<_>>().join("\t") + "\n"
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    read_bound(path, limit).map(|x| x.0)
}
fn read_bound(path: &Path, limit: usize) -> Result<(Vec<u8>, String), String> {
    let mut f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))?;
    let m = f.metadata().map_err(|e| e.to_string())?;
    if !m.is_file() || m.len() > limit as u64 {
        return Err("nonregular or oversized input".into());
    }
    let mut b = Vec::new();
    Read::by_ref(&mut f)
        .take(limit as u64 + 1)
        .read_to_end(&mut b)
        .map_err(|e| e.to_string())?;
    let after = f.metadata().map_err(|e| e.to_string())?;
    if b.len() > limit
        || m.ino() != after.ino()
        || m.len() != after.len()
        || m.mtime() != after.mtime()
        || m.mtime_nsec() != after.mtime_nsec()
        || m.ctime() != after.ctime()
        || m.ctime_nsec() != after.ctime_nsec()
    {
        return Err("input changed during read".into());
    }
    Ok((b, format!("{:?}", after)))
}
/// Captured standalone source has no universal byte ceiling. Native parsers
/// need a whole source, so current host headroom decides whether this attempt
/// can hold it; a pressure refusal is retryable after resources recover.
fn read_adaptive(path: &Path) -> Result<Vec<u8>, String> {
    let host = resources::current().ok_or("source headroom unavailable; retry when host observation recovers")?;
    let limit = usize::try_from(host.work_bytes() / 32).unwrap_or(usize::MAX - 1);
    if limit == 0 { return Err("source capture paused by current memory pressure".into()); }
    if fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.len() > limit as u64) {
        return Err("source exceeds current device headroom for a whole native parser; retry after resources recover".into());
    }
    read(path, limit)
}
fn text(path: &Path) -> Result<String, String> {
    String::from_utf8(read(path, MAX)?).map_err(|_| "invalid UTF-8".into())
}
fn core() -> Result<PathBuf, String> {
    #[cfg(test)]
    {
        let built = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/ug-core");
        if built.is_file() { return Ok(built); }
    }
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let path = exe.parent().ok_or("binary directory")?.join("ug-core");
    if !path.is_file() {
        return Err("compiled ug-core must sit beside ultragoal; run build.py".into());
    }
    Ok(path)
}
fn evaluate(frame: String) -> Result<Vec<Vec<String>>, String> {
    evaluate_until(frame, Instant::now() + Duration::from_secs(30))
}
/// PLAN's read decisions are independent for each F row. Feed every file
/// exactly once while keeping the contract identical on every bounded page.
fn plan_reads(contract: &str, observed: &inventory::Observation) -> Result<Vec<Vec<String>>, String> {
    let head = format!("PLAN\n{contract}\n");
    let mut page = head.clone();
    let mut answers = Vec::new();
    let forced = env::var("UG_PLAN_PAGE_BYTES").ok().and_then(|v| v.parse::<usize>().ok());
    let mut limit = resources::current().map(|h| h.frame_bytes(MAX)).unwrap_or(MAX)
        .min(forced.unwrap_or(MAX)).min(MAX);
    if limit == 0 { return Err("read planning paused by current memory pressure; retry after headroom recovers".into()); }
    for path in observed.files.keys() {
        let file = row(&["F", path, "unread", ""]);
        if page.len() > head.len() && page.len() + file.len() > limit {
            answers.extend(evaluate(page)?);
            page = head.clone();
            limit = resources::current().map(|h| h.frame_bytes(MAX)).unwrap_or(MAX)
                .min(forced.unwrap_or(MAX)).min(MAX);
            if limit == 0 { return Err("read planning paused by current memory pressure; retry after headroom recovers".into()); }
        }
        page += &file;
        if page.len() > MAX { return Err("one read-planning row exceeds the physical core frame".into()); }
    }
    if page.len() > head.len() { answers.extend(evaluate(page)?); }
    if answers.iter().any(|r| r.first().is_some_and(|x| x == "ERROR")) { return Err("Bend read plan refused".into()); }
    Ok(answers)
}
/// Keep the complete sorted listing and every chunk partial in one Bend check
/// decision while transporting the rows through bounded physical frames.
fn evaluate_check_frame(frame: String, operation: &str) -> Result<Vec<Vec<String>>, String> {
    if frame.len() <= MAX && env::var_os("UG_CHECK_PAGE_BYTES").is_none() { return evaluate(frame); }
    let mut lane = core_session::Lane::new("check-pages")?;
    let mut send = |fields: &[&str], limit: usize| -> Result<Vec<Vec<String>>, String> {
        decode_core_rows(lane.request_one(row(fields), limit)?)
    };
    let count = frame.lines().filter(|line| !line.is_empty()).count();
    if send(&["CHECK_BEGIN", operation, &count.to_string()], 1024)? != vec![vec!["CHECK_ACK".to_string(), "begin".to_string()]] {
        return Err("Bend check begin refused".into());
    }
    let forced = env::var("UG_CHECK_PAGE_BYTES").ok().and_then(|v| v.parse::<usize>().ok());
    let (mut sequence, mut page) = (0usize, String::new());
    let mut limit = resources::current().map(|h| h.frame_bytes(MAX)).unwrap_or(MAX)
        .min(forced.unwrap_or(MAX)).min(MAX);
    if limit == 0 { return Err("check paused by current memory pressure; retry after headroom recovers".into()); }
    for line in frame.lines() {
        let item = format!("{line}\n");
        if !page.is_empty() && row(&["CHECK_PAGE", operation, &sequence.to_string(), &(page.clone() + &item)]).len() > limit {
            if send(&["CHECK_PAGE", operation, &sequence.to_string(), &page], 1024)? != vec![vec!["CHECK_ACK".to_string(), "page".to_string()]] {
                return Err("Bend check page refused".into());
            }
            sequence += 1;
            page.clear();
            limit = resources::current().map(|h| h.frame_bytes(MAX)).unwrap_or(MAX)
                .min(forced.unwrap_or(MAX)).min(MAX);
            if limit == 0 { return Err("check paused by current memory pressure; retry after headroom recovers".into()); }
        }
        page += &item;
        if row(&["CHECK_PAGE", operation, &sequence.to_string(), &page]).len() > MAX {
            let _ = send(&["CHECK_ABORT"], 1024);
            return Err("one check row exceeds the physical core frame".into());
        }
    }
    if !page.is_empty() {
        if send(&["CHECK_PAGE", operation, &sequence.to_string(), &page], 1024)? != vec![vec!["CHECK_ACK".to_string(), "page".to_string()]] {
            return Err("Bend check page refused".into());
        }
        sequence += 1;
    }
    let result = send(&["CHECK_END", operation, &sequence.to_string()], REPORT_MAX)?;
    lane.finish()?;
    if let Some(error) = result.iter().find(|r| r.first().is_some_and(|x| x == "CHECK_ERROR" || x == "RANK_ERROR")) {
        return Err(format!("Bend check end refused: {error:?}"));
    }
    Ok(result)
}
fn evaluate_until(frame: String, deadline: Instant) -> Result<Vec<Vec<String>>, String> {
    if frame.len() > MAX {
        return Err("wire frame exceeds 16 MiB".into());
    }
    let phase = match frame.split(['\n', '\t']).next() {
        Some("UG") => "evaluation".to_string(),
        tag => tag.unwrap_or_default().chars().take(32).collect(),
    };
    let output = core_session::request(vec![frame], false, deadline, MAX)
        .map_err(|e| if e.contains("deadline exceeded") { format!("{phase} request: {e}") } else { e })?;
    decode_core_rows(output)
}
fn decode_core_rows(output: Vec<u8>) -> Result<Vec<Vec<String>>, String> {
    String::from_utf8(output)
        .map_err(|_| "core UTF-8".to_string())?
        .lines()
        .map(|line| line.split('\t').map(dec).collect())
        .collect()
}
fn option(args: &[String], name: &str) -> Result<Option<String>, String> {
    match args.iter().position(|s| s == name) {
        None => Ok(None),
        Some(i) => args
            .get(i + 1)
            .filter(|v| !v.starts_with("--"))
            .cloned()
            .map(Some)
            .ok_or(format!("missing {name} value")),
    }
}

fn load_advisory_cache(cache: Option<&String>, namespace: &str) -> Result<(Option<String>, BTreeMap<String,String>, bool),String> {
    let mut prior: BTreeMap<String, String> = BTreeMap::new();
    let cache_core_identity = cache.map(|_| read(&core()?, MAX).map(|b|hash(&b))).transpose()?;
    let mut prior_complete = false;
    if let Some(dir) = cache {
        let p = Path::new(dir).join(format!("{namespace}.json"));
        if p.exists()
            && let Ok(bytes) = read(&p, MAX)
            && let Ok(v) = serde_json::from_slice::<Value>(&bytes)
            && v["core"].as_str() == cache_core_identity.as_deref()
        {
            let payload = serde_json::to_vec(&v["entries"]).map_err(|e| e.to_string())?;
            if v["checksum"] == hash(&payload) {
                if let Ok(entries) = serde_json::from_value(v["entries"].clone()) {
                    prior = entries; prior_complete = true;
                }
            }
        }
        // Untrusted cache is compared after computation, never fed as evidence.
    }
    Ok((cache_core_identity,prior,prior_complete))
}

fn cache_path(value: String, root: &Path) -> Result<String, String> {
    let input = PathBuf::from(value);
    if input
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err("cache path cannot contain parent traversal".into());
    }
    let mut existing = if input.is_absolute() {
        input
    } else {
        env::current_dir().map_err(|e| e.to_string())?.join(input)
    };
    let mut missing = Vec::new();
    while !existing.exists() {
        missing.push(existing.file_name().ok_or("cache path")?.to_os_string());
        existing = existing.parent().ok_or("cache parent")?.to_path_buf();
    }
    let mut resolved = fs::canonicalize(existing).map_err(|e| e.to_string())?;
    for p in missing.into_iter().rev() {
        resolved.push(p)
    }
    if resolved.starts_with(root) {
        return Err("cache must be outside observed source root, including symlink targets".into());
    }
    Ok(resolved.to_str().ok_or("cache UTF-8")?.to_string())
}
fn op_id() -> Result<String, String> {
    let mut nonce = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut nonce))
        .map_err(|e| format!("operation identity unavailable: {e}"))?;
    Ok(nonce.iter().map(|b| format!("{b:02x}")).collect())
}
fn main() {
    // Only set a signal flag here; process cleanup and output stay in normal code.
    unsafe {
        libc::signal(libc::SIGINT, interrupted as *const () as libc::sighandler_t);
        libc::signal(
            libc::SIGTERM,
            interrupted as *const () as libc::sighandler_t,
        );
    }
    let result = dispatch().map(|(mut value,code)|{
        if let Some(session)=session_transport::report(){value["computation_session"]=session;}
        (value,code)
    });
    let core_close = core_session::finish();
    // A failure after index fact sets were written still ends that JSON object.
    let streamed = STREAMED.lock().ok().and_then(|mut s| s.take());
    if is_interrupted() {
        emit(
            streamed,
            &json!({"schema":"ultragoal/1","state":"cancelled","error_code":"cancelled","admission":"unavailable after host cancellation","partial":result.ok().map(|r|r.0),"cleanup_error":core_close.err()}).to_string(),
        );
        eprintln!("error: cancelled");
        std::process::exit(130)
    }
    if let Err(error) = core_close {
        emit(
            streamed,
            &json!({"schema":"ultragoal/1","state":"unknown","error_code":"core_unavailable","error":error,"partial":result.ok().map(|r|r.0)}).to_string(),
        );
        eprintln!("error: core_unavailable");
        std::process::exit(3)
    }
    match result {
        Ok((v, code)) => {
            let serialization_started = Instant::now();
            let output = serde_json::to_string_pretty(&v).unwrap();
            let serialization_us = serialization_started.elapsed().as_micros();
            if streamed.is_none() && output.len() > REPORT_MAX {
                emit(
                    None,
                    &json!({"schema":"ultragoal/1","state":"unknown","error_code":"resource_bound","error":"report exceeds256MiB bound; narrow the selected scope"}).to_string(),
                );
                eprintln!("error: resource_bound");
                std::process::exit(3);
            }
            let output_started = Instant::now();
            let stdout_bytes = emit(streamed, &output);
            if env::args().any(|arg| arg == "--events") {
                eprintln!(
                    "{}",
                    json!({"event":"report-output","operation":v.get("operation"),"serialization_us":serialization_us,"stdout_write_us":output_started.elapsed().as_micros(),"stdout_bytes":stdout_bytes})
                );
            }
            std::process::exit(code)
        }
        Err(e) => {
            let code = e.code();
            emit(streamed, &json!({"schema":"ultragoal/1","state":"unknown","error_code":code,"error":e.message()}).to_string());
            eprintln!("error: {code}");
            std::process::exit(3)
        }
    }
}

/// Only source, pressure, cancellation and credential failures cross this
/// consequential handoff as closed machine states. Legacy free text remains
/// diagnostic context and cannot change any of these four categories.
#[derive(Debug)]
enum Failure {
    InputChanged(String),
    ResourcePressure { path: String, offset: u64 },
    Cancelled(String),
    Legacy(String),
}
impl From<String> for Failure { fn from(value: String) -> Self { Self::Legacy(value) } }
impl From<&str> for Failure { fn from(value: &str) -> Self { Self::Legacy(value.into()) } }
impl Failure {
    fn from_segment(error: segmented::EvalError) -> Self {
        match error {
            segmented::EvalError::Source(fs_adapter::StreamError::Changed) | segmented::EvalError::SourceChanged(_) => Self::InputChanged(error.to_string()),
            segmented::EvalError::Source(fs_adapter::StreamError::Cancelled) => Self::Cancelled(error.to_string()),
            segmented::EvalError::Source(fs_adapter::StreamError::Pressure { offset }) => Self::ResourcePressure { path: "logical source".into(), offset },
            _ => Self::Legacy(error.to_string()),
        }
    }
    fn code(&self) -> &'static str {
        match self {
            Self::InputChanged(_) => "input_changed",
            Self::ResourcePressure { .. } => "resource_bound",
            Self::Cancelled(_) => "cancelled",
            Self::Legacy(text) => error_code(text),
        }
    }
    fn message(&self) -> String {
        match self {
            Self::InputChanged(text) | Self::Cancelled(text) | Self::Legacy(text) => text.clone(),
            Self::ResourcePressure { path, offset } => format!("{path}: current memory pressure at byte {offset}; retry from fresh source"),
        }
    }
}

/// Stable machine-readable class of a failure message; exit numbers are unchanged.
/// The first matching rule wins, so specific classes precede general ones.
fn error_code(error: &str) -> &'static str {
    // Core stderr and path lists are free text, so only the failure itself is classified.
    let error = ["; core diagnosis: ", "; paths "].iter().filter_map(|m| error.find(m)).min().map_or(error, |i| &error[..i]);
    const RULES: &[(&str, &[&str])] = &[
        ("deadline_exceeded", &["request deadline exceeded", "observation deadline exceeded"]),
        ("invalid_arguments", &["unknown option", "unknown command", " required", "conflicts with", "missing --", "budget refused", "lifetime refused", "sun_path limit"]),
        ("cancelled", &["cancelled"]),
        ("credential_refused", &["credential"]),
        ("disclosure_refused", &["disclosure"]),
        ("resource_bound", &["exceeds", "bound", "limit", "oversized"]),
        ("input_changed", &["changed during", "stale"]),
        ("session_unavailable", &["explicit session", "session socket refused", "start a new session", "session endpoint was replaced", "session workspace/endpoint/core changed"]),
        ("core_unavailable", &["ug-core", "core session", "core response", "core closed", "core did not", "core input closed", "core output closed", "core std", "core termination", "core transport", "core writer", "owned core", "lane core", "Bend computation identity"]),
        ("input_unavailable", &["No such file", "not found", "unavailable", "nonregular", "Permission denied"]),
        ("invalid_input", &["invalid", "malformed", "UTF-8", "shape", "schema", "duplicate", "expected", "missing"]),
    ];
    RULES
        .iter()
        .find(|(_, needles)| needles.iter().any(|n| error.contains(n)))
        .map_or("operation_failed", |(code, _)| code)
}
fn content_chunk_report(c: &chunks::Chunked, captured: &inventory::Observation) -> Result<Value, String> {
    let (mut omitted, mut parsed, mut evaluated) = (Vec::<Value>::new(), 0u64, 0u64);
    // Each CH row is followed by its CF rows, which name the chunk's files.
    let mut in_omitted = false;
    let mut chunk_id = String::new();
    for line in c.rows.lines().filter(|l| l.starts_with("CH\t") || l.starts_with("CF\t")) {
        let r = line.split('\t').map(dec).collect::<Result<Vec<_>, _>>()?;
        if r[0] == "CF" {
            if in_omitted {
                omitted.last_mut().ok_or("chunk listing")?["paths"].as_array_mut().ok_or("chunk listing")?.push(json!(r[1]));
            } else if let Some((_, reason)) = captured.problems.get(&r[1]) {
                // Bend completed the physical CHUNK request, but this source
                // contributed no content. Keep its named omission visible.
                omitted.push(json!({"chunk":chunk_id,"reason":reason,"bytes":0,"paths":[r[1]]}));
            }
            continue;
        }
        if r.len() != 7 {
            return Err("invalid content chunk row".into());
        }
        chunk_id = r[1].clone();
        in_omitted = r[2] != "complete";
        if in_omitted {
            omitted.push(json!({"chunk":r[1],"reason":r[3],"bytes":r[4],"paths":[]}));
        } else {
            evaluated += r[4].parse::<u64>().map_err(|_| "chunk byte count")?;
        }
        parsed += r[6].parse::<u64>().map_err(|_| "chunk parse count")?;
    }
    Ok(json!({"mode":"content streamed in chunk requests on dedicated cores; Bend computes each chunk partial and folds them","frames":c.frames,"core_requests":c.requests,"core_processes":c.cores,"evaluated_bytes":evaluated,"parsed":parsed,"omitted":omitted,"fact_sets":"not reported for chunked content; use index"}))
}

#[cfg(test)]
mod content_chunk_report_tests {
    use super::*;
    #[test]
    fn pressured_file_is_named_as_omitted_even_when_chunk_transport_completed() {
        let mut problems = BTreeMap::new();
        problems.insert("b.txt".into(), ("file".into(), "ResourcePressure at byte 4096; retry from fresh source".into()));
        let observed = inventory::Observation {
            files: BTreeMap::new(), identities: BTreeMap::new(), kinds: BTreeMap::new(),
            problems, issues: Vec::new(), complete: true, root_identity: None, exclusions: Vec::new(),
        };
        let chunks = chunks::Chunked { rows: row(&["CH", "0", "complete", "", "13", "1", "0"])
            + &row(&["CF", "a.txt", "digest"])
            + &row(&["CH", "1", "complete", "", "0", "1", "0"])
            + &row(&["CF", "b.txt", "unavailable"]), frames: 2, requests: 2, cores: 1, ms: 0 };
        let report = content_chunk_report(&chunks, &observed).unwrap();
        assert_eq!(report["evaluated_bytes"], 13);
        assert_eq!(report["omitted"][0]["paths"][0], "b.txt");
        assert_eq!(report["omitted"][0]["reason"], "ResourcePressure at byte 4096; retry from fresh source");
    }
}

fn dispatch() -> Result<(Value, i32), Failure> {
    let args: Vec<String> = env::args().skip(1).collect();
    validate_arguments(&args)?;
    let verb = args.first().map(String::as_str).unwrap_or("help");
    session_transport::configure(&args)?;
    let Some(mut path) = option(&args, "--journal")? else {
        return dispatch_inner();
    };
    if !["check", "verify", "semantic"].contains(&verb) {
        return dispatch_inner();
    }
    if let Some(root) = option(&args, "--root")? {
        path = cache_path(path, &fs::canonicalize(root).map_err(|e| e.to_string())?)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&path)
        .map_err(|e| format!("journal create: {e}"))?;
    let operation = op_id()?;
    file.write_all(
        row(&[
            "EVENT",
            &operation,
            "running",
            &hash(&serde_json::to_vec(&args).map_err(|e| e.to_string())?),
        ])
        .as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    let outcome = dispatch_inner();
    let payload = match &outcome {
        Ok((v, _)) => v.clone(),
        Err(e) => json!({"state":"unknown","error_code":e.code(),"error":e.message()}),
    };
    let fingerprint = hash(&serde_json::to_vec(&payload).map_err(|e| e.to_string())?);
    let event = if is_interrupted() {
        "cancelled"
    } else {
        "completed"
    };
    if let Err(e) = file
        .write_all(row(&["EVENT", &operation, event, &fingerprint]).as_bytes())
        .and_then(|_| file.sync_all())
    {
        return Ok((
            json!({"schema":"ultragoal/1","state":"unknown","reason":"journal terminal write failed","detail":e.to_string(),"partial":payload}),
            2,
        ));
    }
    outcome.map(|(mut v,code)|{v["journal"]=json!({"path":path,"operation":operation,"payload_sha256":fingerprint,"hash_excludes_journal_field":true,"authority":"reported only"});(v,code)})
}

fn validate_arguments(args: &[String]) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut i = 1;
    while i < args.len() {
        let flag = &args[i];
        if !seen.insert(flag) {
            return Err(format!("duplicate option {flag}"));
        }
        match flag.as_str() {
            "--local" | "--no-cache" | "--semantic" | "--events" | "--details" | "--brief" => i += 1,
            "--root" | "--contract" | "--cache" | "--report" | "--id" | "--request"
            | "--response" | "--disclosure" | "--keychain-service" | "--stdin-c" | "--rust"
            | "--python" | "--typescript" | "--journal" | "--baseline" | "--scope" | "--query"
            | "--prior" | "--bend-proof" | "--session" | "--directory" | "--shortlist"
            | "--semantic-deadline-ms" | "--semantic-concurrency" | "--semantic-max-requests" | "--tools"
            | "--usage-ledger" | "--lifetime-seconds" | "--idle-seconds" | "--semantic-blocking" => {
                option(args, flag)?;
                i += 2
            }
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    if seen.contains(&"--semantic-deadline-ms".to_string()) || seen.contains(&"--semantic-concurrency".to_string()) {
        provider::budget(args)?;
    }
    if seen.contains(&"--local".to_string()) && seen.contains(&"--semantic".to_string()) {
        return Err("--local conflicts with --semantic".into());
    }
    if !matches!(option(args, "--semantic-blocking")?.as_deref(), None | Some("on" | "off")) {
        return Err("--semantic-blocking on|off required".into());
    }
    Ok(())
}

fn dispatch_inner() -> Result<(Value, i32), Failure> {
    let args: Vec<String> = env::args().skip(1).collect();
    let verb = args.first().map(String::as_str).unwrap_or("help");
    validate_arguments(&args)?;
    if verb == "session" { return Ok(session_transport::serve(&args)?); }
    if verb == "verify" && args.contains(&"--stdin-c".to_string()) {
        return Ok(native_probe(&args)?);
    }
    if verb == "verify" && args.contains(&"--rust".to_string()) {
        return Ok(rust_syntax(&args)?);
    }
    if verb == "verify" && args.contains(&"--python".to_string()) {
        return Ok(native_syntax(&args, "python")?);
    }
    if verb == "verify" && args.contains(&"--typescript".to_string()) {
        return Ok(native_syntax(&args, "typescript")?);
    }
    if verb == "verify" && args.contains(&"--bend-proof".to_string()) {
        return Ok(proof::verify(&args)?);
    }
    if verb == "help" || verb == "--help" {
        return Ok((
            json!({"schema":"ultragoal/1","commands":["session --root DIR --directory NEW_PRIVATE_DIRECTORY (foreground)","check --root DIR --contract FILE [--cache DIR|--no-cache] [--local] [--session HANDLE.json]","verify --root DIR --contract FILE","verify --rust|--python|--typescript|--stdin-c|--bend-proof FILE","fit --root DIR [--contract PROPOSED_REQUIREMENTS]","index --root DIR [--scope SELECTOR] --no-cache","select --root DIR --report INDEX.json --query TEXT [--shortlist N] [--prior FILE] [--details]","inspect","explain --report FILE [--id ID]","explain --journal FILE","semantic --request FILE --disclosure synthetic|public|project [--keychain-service NAME]","advise --request CONTEXT.json --disclosure CLASS [--keychain-service NAME]","semantic --request FILE --response FILE (offline reported validation)"],"writes":"Explicit cache/journal destinations and explicitly started session endpoints only; no automatic target mutation, installation or adoption.","support":"Bend owns bounded contract parsing and obligation admission. Text/membership checks are exact only for their stated predicates; syntax, runtime and semantic claims remain distinct."}),
            0,
        ));
    }
    if verb == "inspect" {
        return Ok((
            json!({"schema":"ultragoal/1","engine":"Bend","bridge":"Rust","core_sha256":hash(&read(&core()?,MAX)?),"contract":"UG tab-separated version 1; percent escapes %25 %09 %0A %0D","adoption":"advisory; no qualified protected host channel","native":"captured Rust/Python/TypeScript/C11 syntax and bounded local Bend check-only import closures; full project runtime unqualified","parser_coverage":{"json":"full value grammar, lexical numbers, duplicate decoded keys rejected, depth64","rust_python_typescript":"bounded token/delimiter/indentation structure and named-header/import facts with explicit limitations; native syntax adapters separate","markdown":"line candidates with byte spans; not renderer or complete CommonMark parser"},"selection":"optional lexical shortlist (files ranked by BM25F, pooled 4G reranked by best window, first G = 128 groups by default, --shortlist 1..512) judged in head-plus-slices windows, exact spans and retained anchors; --details exposes full candidate metadata","semantic":"advisory, except that a qualified-family contradiction under the fitted policy fails a pending obligation (--semantic-blocking on|off, default on)","frame_limit":MAX,"evaluation_frame_budget":inventory::FRAME_BUDGET,"report_byte_limit":REPORT_MAX,"index_report":"streamed: fact sets are written one per line as the core answers them; the report byte limit applies to other reports and to each core answer","listing_entry_limit":"current host headroom; an incomplete scan is named ResourcePressure","headroom":resources::current().map(|h|json!({"available_bytes":h.available,"physical_bytes":h.physical,"work_bytes":h.work_bytes(),"index_frame_bytes":h.frame_bytes(MAX),"index_lanes":h.index_lanes()})),"index_core_processes":"one core for a single-request index; otherwise one or two lanes chosen from current headroom with fresh cores per request","index_request_listing_bytes":index::REQUEST_BYTES,"core_request_stall_ms":core_session::REQUEST_STALL.as_millis(),"index_bend_threads":8,"index_frames_per_batch":8}),
            0,
        ));
    }
    if verb == "explain" {
        if let Some(path) = option(&args, "--journal")? {
            let observations = evaluate("JOURNAL\n".to_string() + &text(Path::new(&path))?)?;
            return Ok((
                json!({"schema":"ultragoal/1","observations":observations,"authority":"untrusted reported journal; cannot authorize replay or satisfy runtime assurance"}),
                0,
            ));
        }
        let p = option(&args, "--report")?.ok_or("--report required")?;
        let v: Value = serde_json::from_str(&text(Path::new(&p))?).map_err(|e| e.to_string())?;
        if v.get("native_observation").is_some() { return Ok((native_observation::explain_import(v),0)); }
        let id = option(&args, "--id")?;
        let selected = v["obligations"]
            .as_array()
            .ok_or("report obligations missing")?
            .iter()
            .filter(|r| id.as_ref().is_none_or(|id| r["id"] == *id))
            .cloned()
            .collect::<Vec<_>>();
        return Ok((
            json!({"schema":"ultragoal/1","state":"reported","admitted_as_current":false,"obligations":selected,"reported_adoption":v["adoption"],"adoption":"unverified imported report; current checks were not rerun"}),
            0,
        ));
    }
    if verb == "semantic" {
        return Ok(semantic(&args)?);
    }
    if verb == "advise" {
        return Ok(provider::advise(&args)?);
    }
    if verb == "select" {
        return Ok(context::select(&args)?);
    }
    if !["check", "verify", "fit", "index"].contains(&verb) {
        return Err("unknown command".into());
    }
    let root = fs::canonicalize(option(&args, "--root")?.ok_or("--root required")?)
        .map_err(|e| e.to_string())?;
    if verb == "fit" { return Ok(fit::propose(&args,&root)?); }
    let contract = option(&args, "--contract")?.unwrap_or_default();
    if verb != "index" && contract.is_empty() {
        return Err("--contract required".into());
    }
    if verb == "index" && !contract.is_empty() {
        return Err("index creates optional facts only; use check to evaluate a contract".into());
    }
    let contract_text = if verb == "index" {
        "UG\t1\n".to_string() + &row(&["Q", option(&args, "--scope")?.as_deref().unwrap_or("*")])
    } else {
        text(Path::new(&contract))?
    };
    let plan_started = Instant::now();
    let contract_validation = if verb == "index" {
        vec![vec!["VALID".to_string()]]
    } else {
        evaluate(format!("CONTRACT_PLAN\n{contract_text}"))?
    };
    if contract_validation.first() != Some(&vec!["VALID".to_string()]) {
        return Err("Bend rejected contract grammar, version, duplicate ID, empty requirement or injected transport row".into());
    }
    let preservation = if let Some(path) = option(&args, "--baseline")? {
        let baseline = text(Path::new(&path))?;
        let valid = evaluate(format!("CONTRACT\n{baseline}"))?;
        if valid != vec![vec!["VALID".to_string()]] {
            return Err("invalid baseline contract".into());
        }
        Some(
            evaluate("BASELINE\n".to_string() + &row(&[&baseline, &contract_text]))?
                == vec![vec!["BASELINE".to_string(), "preserved".to_string()]],
        )
    } else {
        None
    };
    let contract_plan_ms = plan_started.elapsed().as_millis();
    let started = Instant::now();
    let observer = inventory::Session::open(&root)?;
    let baseline = observer.baseline()?;
    let listing = baseline.listing();
    let mut discovered = observer.capture(listing, None)?;
    let discovery_ms = started.elapsed().as_millis();
    let plan_start = Instant::now();
    let direct = verb != "index"
        && !contract_validation
            .iter()
            .any(|r| r.first().is_some_and(|s| s == "PLAN_NEEDS_MEMBERSHIP"));
    let plan = if direct {
        contract_validation.into_iter().skip(1).collect()
    } else {
        plan_reads(&contract_text, &discovered)?
    };
    let plan_ms = plan_start.elapsed().as_millis();
    let mut wanted = std::collections::BTreeSet::new();
    let mut native_wanted = std::collections::BTreeSet::new();
    let mut semantic_wanted = std::collections::BTreeSet::new();
    for r in plan {
        if r.len() != 2
            || !matches!(
                r[0].as_str(),
                "READ" | "NATIVE_READ" | "SEMANTIC_READ" | "READ_IF_FILE" | "NATIVE_READ_IF_FILE" | "SEMANTIC_READ_IF_FILE"
            )
        {
            return Err("invalid Bend read plan".into());
        }
        if r[0].ends_with("_IF_FILE")
            && discovered.kinds.get(&r[1]).map(String::as_str) != Some("file")
        {
            continue;
        }
        wanted.insert(r[1].clone());
        if matches!(r[0].as_str(), "NATIVE_READ" | "NATIVE_READ_IF_FILE") {
            native_wanted.insert(r[1].clone());
        }
        if matches!(r[0].as_str(), "SEMANTIC_READ" | "SEMANTIC_READ_IF_FILE") {
            semantic_wanted.insert(r[1].clone());
        }
    }
    let has_segmented_source = wanted.iter().any(|path| listing.members.iter().any(|m|
        m.path == path.as_bytes() && m.identity.size > (MAX / 3) as u64));
    let segmented_path = (wanted.len() == 1).then(|| {
        wanted.iter().find(|path| listing.members.iter().any(|m|
            m.path == path.as_bytes() && m.identity.size > (MAX / 3) as u64)).cloned()
    }).flatten();
    let chunked = verb != "index" && chunks::selected_bytes(listing, &wanted) > chunks::THRESHOLD;
    let chunk_header = format!("CHUNK\n{contract_text}\n");
    // Semantic requests need their actual selected source. The final check
    // now carries bounded pages, so cumulative semantic content is not pruned
    // merely to fit one physical frame.
    let kept = if chunked {
        semantic_wanted.clone()
    } else {
        std::collections::BTreeSet::new()
    };
    let operation = op_id()?;
    let events = args.contains(&"--events".to_string());
    let mut frame = contract_text.clone();
    frame.push('\n');
    frame += &row(&[
        "OP",
        &operation,
        "running",
        &hash(contract_text.as_bytes()),
        "timely",
    ]);
    if let Some(n) = option(&args, "--semantic-max-requests")? {
        frame += &row(&["SB", &n]);
    }
    // Only whole-frame planning runs before capture: subtrees past the frame budget
    // are never read and are named instead.
    let planned = (verb != "index" && !has_segmented_source).then(|| {
        let partials = contract_text
            .lines()
            .filter(|l| l.starts_with("O\t"))
            .map(|l| 24 + l.split('\t').enumerate().filter(|(i, _)| matches!(i, 5 | 7 | 8)).map(|(_, f)| f.len()).sum::<usize>())
            .sum();
        let shape = inventory::FrameShape { wanted: &wanted, native: &native_wanted, kept: &kept, chunked, fixed: frame.len(), partials };
        discovered.plan_frame(&shape, inventory::FRAME_BUDGET)
    });
    let omitted: Vec<String> = Vec::new();
    let read_start = Instant::now();
    let mut native_ms = 0u128;
    let mut native_row = |path: &str, bytes: &[u8]| -> Result<String, String> {
        let started = Instant::now();
        let result = if !path.ends_with(".rs") {
            "unknown"
        } else if native_syntax::rust(std::str::from_utf8(bytes).map_err(|_| "native UTF-8")?).is_ok() {
            "verified"
        } else {
            "failed"
        };
        native_ms += started.elapsed().as_millis();
        Ok(row(&["N", path, &hash(bytes), result, &operation, "syn-2.0.117", native_identity::RUST_SYNTAX]))
    };
    let mut content_chunks = None;
    let mut index_rows = None;
    let mut segmented_ms = 0u128;
    let mut segmented_stats = Value::Null;
    // Parsed and reused fact sets of a streamed index.
    let mut streamed_work = (0usize, 0usize);
    let direct_segment = if let Some(path) = &segmented_path {
        let (prefix, suffix) = segmented::frame_halves(&frame, &discovered, &wanted, path, verb == "index")?;
        let command = if verb == "index" { "SEG_INDEX" } else { "SEG_EVAL" };
        (row(&[command, &operation, path, &"0".repeat(64), &"0".repeat(64), &prefix, &suffix]).len() <= MAX)
            .then_some((path.clone(), prefix, suffix))
    } else { None };
    let (captured, total) = if let Some((path, prefix, suffix)) = direct_segment {
        if verb == "index" { Streamed::open()?; }
        let segment_started = Instant::now();
        let result = segmented::evaluate_one(&observer, &path, &operation, &prefix, &suffix, if verb == "index" { segmented::Mode::Index } else { segmented::Mode::Check }).map_err(Failure::from_segment)?;
        segmented_ms = segment_started.elapsed().as_millis();
        if discovered.identities.get(&path) != Some(&result.source.identity) {
            return Err(Failure::InputChanged("logical source changed since membership listing".into()));
        }
        segmented_stats = json!({"mode":"one logical source in ordered Bend pages","pages":result.source.pages,"bytes":result.source.bytes,"core_processes":result.cores,"sha256":result.source.digest});
        let total = usize::try_from(result.source.bytes).map_err(|_| "source size exceeds host address space")?;
        let mut rows = Vec::new();
        for r in result.rows {
            if verb == "index" && matches!(r.first().map(String::as_str), Some("P" | "P_SPANS" | "P_MIXED")) {
                let set = fact_set(&r)?;
                streamed_work.0 += usize::from(set["computation"] == "parsed");
                streamed_work.1 += usize::from(set["computation"] == "reused");
                Streamed::write(&set)?;
            } else {
                rows.push(r);
            }
        }
        index_rows = Some(rows);
        (discovered, total)
    } else if has_segmented_source && verb != "index" {
        let (result, total) = segmented::mixed_chunks(&observer, listing, &mut discovered, &wanted, &contract_text, &operation).map_err(Failure::from_segment)?;
        frame += &discovered.head();
        frame += &row(&["FC"]);
        frame += &discovered.tail();
        frame += &result.rows;
        content_chunks = Some(result);
        (discovered, total)
    } else if verb == "index" {
        Streamed::open()?;
        let mut rows = Vec::new();
        let total = index::evaluate_index(&observer, listing, &mut discovered, &wanted, &contract_text, &operation, &mut |answer| {
            let answer = String::from_utf8(answer).map_err(|_| "core UTF-8")?;
            for line in answer.lines() {
                let r = line.split('\t').map(dec).collect::<Result<Vec<_>, _>>()?;
                match r[0].as_str() {
                    "P" | "P_SPANS" | "P_MIXED" => {
                        let set = fact_set(&r)?;
                        streamed_work.0 += usize::from(set["computation"] == "parsed");
                        streamed_work.1 += usize::from(set["computation"] == "reused");
                        Streamed::write(&set)?;
                    }
                    "ERROR" => return Err(format!("Bend rejected input: {r:?}")),
                    _ => rows.push(r),
                }
            }
            Ok(())
        })?;
        index_rows = Some(rows);
        (discovered, total)
    } else if chunked {
        let mut stream = chunks::Stream::new(chunk_header.clone());
        let (mut files, mut native, mut total) = (String::new(), String::new(), 0usize);
        observer.stream(listing, &mut discovered, &wanted, |observed, path, bytes| {
            let mut content = std::str::from_utf8(&bytes).unwrap_or("");
            if !content.is_empty() && chunks::oversized(chunk_header.len(), path, content) {
                observed.problems.insert(path.into(), ("file".into(), "exceeds content chunk frame bound".into()));
                content = "";
            }
            let digest = observed.digest(path, &wanted, &bytes);
            stream.push(path, row(&["F", path, &digest, content]))?;
            if kept.contains(path) {
                files += &row(&["F", path, &digest, content]);
            }
            if !observed.problems.contains_key(path) {
                total += bytes.len();
                if native_wanted.contains(path) {
                    native += &native_row(path, &bytes)?;
                }
            }
            Ok(())
        })?;
        let result = stream.finish()?;
        // Bend expands the chunks' CF listing at the FC marker; only kept files carry content here.
        frame += &discovered.head();
        frame += &row(&["FC"]);
        frame += &files;
        frame += &discovered.tail();
        frame += &result.rows;
        frame += &native;
        content_chunks = Some(result);
        (discovered, total)
    } else {
        let mut captured = observer.capture(listing, Some(&wanted))?;
        captured.omit(&omitted);
        frame += &captured.frame(&wanted);
        for path in &native_wanted {
            if let Some(bytes) = captured.files.get(path).filter(|_| !captured.problems.contains_key(path)) {
                frame += &native_row(path, bytes)?;
            }
        }
        let total = captured.files.values().map(Vec::len).sum();
        (captured, total)
    };
    let native_parse_ms = native_ms;
    let files = &captured.files;
    let capture_ms = started.elapsed().as_millis();
    let read_ms = read_start.elapsed().as_millis();
    if events {
        eprintln!(
            "{}",
            json!({"event":"captured","operation":operation,"files":files.len(),"selected_bytes":total,"evaluation_frame":planned.as_ref().map(|p| json!({"bytes":frame.len(),"estimated_bytes":p.frame,"estimated_key_bytes":p.key,"single_frame_budget":inventory::FRAME_BUDGET,"single_frame_would_omit":p.omitted,"actual_omitted":omitted}))})
        );
    }
    let cache = option(&args, "--cache")?
        .map(|p| cache_path(p, &root))
        .transpose()?;
    if cache.is_some() && args.contains(&"--no-cache".to_string()) {
        return Err("--cache conflicts with --no-cache".into());
    }
    let namespace = hash(root.to_string_lossy().as_bytes());
    if is_interrupted() { return Err("cancelled before publication".into()); }
    // Native byte hashing and cache reads are independent of pure evaluation.
    // Scoped joining retains ownership even when either operation fails.
    let (rows,core_ms,(cache_core_identity,prior,prior_complete)) = std::thread::scope(|scope| {
        let loader=cache.as_ref().map(|_|scope.spawn(||load_advisory_cache(cache.as_ref(),&namespace)));
        let before_core=Instant::now();
        let rows=match index_rows { Some(rows)=>Ok(rows), None=>evaluate_check_frame(frame,&operation) };
        let core_ms=if segmented_ms > 0 { segmented_ms } else if verb=="index" { read_ms } else { before_core.elapsed().as_millis() };
        let cached=match loader {
            Some(handle)=>handle.join().map_err(|_|"cache reader panicked".to_string())?,
            None=>Ok((None,BTreeMap::new(),false)),
        };
        Ok::<_,String>((rows?,core_ms,cached?))
    })?;
    if rows.iter().any(|r| r.first().is_some_and(|s| s == "ERROR")) {
        return Err(format!("Bend rejected input: {rows:?}").into());
    }
    let mut obligations = Vec::new();
    let mut next = BTreeMap::new();
    let mut fact_sets:Vec<_>=rows.iter().filter(|r|r.first().is_some_and(|s|matches!(s.as_str(),"P"|"P_SPANS"|"P_MIXED"))).map(|r|fact_set(r)).collect::<Result<Vec<_>,String>>()?;
    let parsed_count = streamed_work.0 + fact_sets
        .iter()
        .filter(|f| f["computation"] == "parsed")
        .count();
    let reused_count = streamed_work.1 + fact_sets
        .iter()
        .filter(|f| f["computation"] == "reused")
        .count();
    for r in rows
        .iter()
        .filter(|r| r.first().is_some_and(|s| s == "RESULT"))
    {
        if r.len() != 15 {
            return Err("invalid Bend result envelope".into());
        }
        let digest = digest_id(&r[6])?;
        let cache = if prior.get(&digest) == Some(&r[2]) {
            "advisory-match"
        } else {
            "miss"
        };
        next.insert(digest.clone(), r[2].clone());
        let coverage = rows.iter().find(|c| c.len() == 7 && c[0] == "CHUNK_COVERAGE" && c[1] == r[1]);
        let witness = rows.iter().find(|w| w.len() == 4 && w[0] == "WITNESS" && w[1] == r[1]);
        obligations.push(json!({"id":r[1],"requirement_revision":r[12],"state":r[2],"requirement":r[3],"origin":r[4],"next_action":r[5],"computation_key":digest,"cache":cache,"computation":r[7],"check":{"selector":r[8],"argument_or_verifier_request":r[9],"required_assurance":r[10],"rule":r[11],"predicate_surface":r[13],"dependency_projection":r[14]},"assurance":"declared predicate surface only; original-outcome and adoption alignment is not certified"}));
        if let Some(c) = coverage {
            let o = obligations.last_mut().ok_or("obligation")?;
            // Field 4 is one nested row naming the in-scope paths of omitted chunks.
            let omitted = c[4].strip_suffix('\n').map_or(Ok(Vec::new()), |r| r.split('\t').map(dec).collect::<Result<Vec<_>, _>>())?;
            o["chunk_coverage"] = json!({"chunks":c[2].parse::<u64>().map_err(|_| "chunk coverage count")?,"evaluated":c[3].parse::<u64>().map_err(|_| "chunk coverage count")?,"omitted":omitted,"unavailable":c[5].parse::<u64>().map_err(|_| "chunk coverage count")?,"partials":c[6]});
        }
        if let Some(w) = witness {
            let o = obligations.last_mut().ok_or("obligation")?;
            o["counterexample"] = json!({"path":w[2],"line":w[3].parse::<u64>().ok()});
        }
    }
    if obligations.is_empty() && verb != "index" {
        return Err("contract has no obligations".into());
    }
    let mut semantic_results: Vec<Value> = rows
        .iter()
        .filter(|r| r.first().is_some_and(|s| s == "SEMANTIC_UNAVAILABLE"))
        .map(|r| json!({"obligation":r.get(1),"state":"unknown","reason":r.get(2)}))
        .collect();
    let requests: Vec<_> = rows
        .iter()
        .filter(|r| r.first().is_some_and(|s| s == "REQUEST"))
        .cloned()
        .collect();
    if !args.contains(&"--local".to_string())
        && option(&args, "--disclosure")?.is_some()
        && !requests.is_empty()
    {
        match provider::assess_rows(&args, &requests) {
            Ok(v) => semantic_results.push(v),
            Err(e) => semantic_results.push(json!({"state":"unknown","reason":e})),
        }
    }
    if let Some(dir) = &cache && !(prior_complete && prior == next) {
        let dir = PathBuf::from(dir);
        if dir.starts_with(&root) {
            return Err("cache must be outside observed source root".into());
        }
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let p = dir.join(format!("{namespace}.json"));
        let tmp = dir.join(format!(".{operation}.tmp"));
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        let entries = serde_json::to_value(&next).map_err(|e| e.to_string())?;
        let checksum = hash(&serde_json::to_vec(&entries).map_err(|e| e.to_string())?);
        f.write_all(
            &serde_json::to_vec(
                &json!({"entries":entries,"checksum":checksum,"core":cache_core_identity}),
            )
            .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        fs::rename(tmp, p).map_err(|e| e.to_string())?;
    }
    let revalidate_started = Instant::now();
    let (mut current_observation, revalidation_mode) = observer.revalidate(&baseline, &wanted)?;
    current_observation.omit(&omitted);
    let revalidate_ms = revalidate_started.elapsed().as_millis();
    let changes = captured.changes(&current_observation);
    let contract_current = preservation != Some(false)
        && (verb == "index"
            || text(Path::new(&contract)).ok().as_deref() == Some(contract_text.as_str()));
    let current = contract_current
        && changes.is_empty()
        && captured.complete
        && current_observation.complete
        && captured.problems.is_empty()
        && current_observation.problems.is_empty();
    let mut admission = format!(
        "REVALIDATE_SCOPED\n{}\n",
        if contract_current {
            "current"
        } else {
            "obsolete"
        }
    );
    for o in &obligations {
        admission += &row(&[
            "R",
            o["id"].as_str().ok_or("id")?,
            o["state"].as_str().ok_or("state")?,
            o["check"]["selector"].as_str().ok_or("selector")?,
            o["check"]["dependency_projection"].as_str().ok_or("dependency projection")?,
            o["check"]["rule"].as_str().ok_or("rule")?,
            o["check"]["argument_or_verifier_request"].as_str().ok_or("argument")?,
            o["check"]["required_assurance"].as_str().ok_or("assurance")?,
            o["counterexample"]["path"].as_str().unwrap_or(""),
            &o["counterexample"]["line"].as_u64().map_or(String::new(), |n| n.to_string()),
        ]);
    }
    for (path, kind) in &changes {
        admission += &row(&["C", path, kind]);
    }
    for (path, _) in &current_observation.issues {
        admission += &row(&["C", path, "incomplete"]);
    }
    for path in current_observation.problems.keys() {
        admission += &row(&["C", path, "unavailable"]);
    }
    let admission_started = Instant::now();
    let admissions = evaluate(admission)?;
    let admission_ms = admission_started.elapsed().as_millis();
    if admissions.len() != obligations.len() {
        return Err("incomplete current admission coverage".into());
    }
    for (o, r) in obligations.iter_mut().zip(admissions) {
        if r.len() != 5 || r[0] != "ADMISSION" || o["id"] != r[1] {
            return Err("current admission identity mismatch".into());
        }
        o["computed_state"] = o["state"].clone();
        o["state"] = json!(r[2]);
        o["next_action"] = json!(r[3]);
        o["admission_reason"] = json!(r[4]);
    }
    // Bend decides whether a semantic decision changes an admitted state; groups
    // without a decision (transport errors) have nothing to admit.
    let blocking = option(&args, "--semantic-blocking")?.unwrap_or_else(|| "on".into());
    let mut semantic_admit = String::new();
    for g in semantic_results.iter().filter_map(|v| v["groups"].as_array()).flatten() {
        let (Some(id), Some(family), Some(d)) = (g["bindings"][0]["obligation"].as_str(), g["bindings"][0]["rubric"].as_str(), g["decision"][0].as_array()) else { continue };
        let o = obligations.iter().find(|o| o["id"] == id).ok_or("semantic decision without obligation")?;
        semantic_admit += &row(&[id, o["state"].as_str().ok_or("state")?, family, d[1].as_str().ok_or("decision state")?, d[2].as_str().ok_or("decision policy")?, &blocking]);
    }
    if !semantic_admit.is_empty() {
        for r in evaluate("SEMANTIC_ADMIT\n".to_string() + &semantic_admit)? {
            if r.len() != 5 || r[0] != "SEMANTIC_ADMISSION" {
                return Err(format!("invalid semantic admission: {}", r.join(" ")).into());
            }
            let o = obligations.iter_mut().find(|o| o["id"] == r[1]).ok_or("semantic admission identity mismatch")?;
            if !r[3].is_empty() {
                o["state"] = json!(r[2]);
                o["next_action"] = json!(r[3]);
                o["admission_reason"] = json!(r[4]);
            }
        }
    }
    let mut summary = format!(
        "{}\n{}\n",
        if verb == "index" {
            "INDEX_SUMMARY"
        } else {
            "SUMMARY"
        },
        if (verb == "index" && current) || (verb != "index" && contract_current) {
            "current"
        } else {
            "obsolete"
        }
    );
    for o in &obligations {
        summary += o["state"].as_str().ok_or("state")?;
        summary.push('\n');
    }
    let summary_started = Instant::now();
    let summary = evaluate(summary)?;
    let summary_ms = summary_started.elapsed().as_millis();
    let state = summary.first().ok_or("missing summary")?;
    if state.len() != 3 || state[0] != "STATE" {
        return Err("invalid summary".into());
    }
    let unresolved = obligations
        .iter()
        .filter(|v| v["state"] != "verified")
        .count();
    let mut changed_concerns = Vec::new();
    for o in &mut obligations {
        let fingerprint = hash(
            &serde_json::to_vec(&json!([
                o["id"],
                o["requirement"],
                o["computation_key"],
                o["state"]
            ]))
            .map_err(|e| e.to_string())?,
        );
        let same = o["cache"] == "advisory-match" && o["state"] == o["computed_state"];
        o["concern_fingerprint"] = json!(fingerprint);
        o["changed_concern"] = json!(o["state"] != "verified" && !same);
        if o["changed_concern"] == true {
            changed_concerns.push(json!({"id":o["id"],"fingerprint":o["concern_fingerprint"],"state":o["state"],"next_action":o["next_action"]}));
        }
    }
    let code = state[2].parse::<i32>().map_err(|_| "summary exit")?;
    let details = verb == "index" || args.contains(&"--details".to_string());
    if !details {
        for set in &mut fact_sets {
            let runs = set["line_runs"].as_array().map_or(0, |runs| runs.iter().filter_map(Value::as_array).map(|run| run.len() - 2).sum());
            let count = set["facts"].as_array().map_or(0, Vec::len) + runs;
            set["fact_count"] = json!(count);
            let set = set.as_object_mut().unwrap();
            set.remove("facts");
            set.remove("line_runs");
        }
    }
    let mut report = json!({"schema":"ultragoal/1","operation":operation,"state":state[1],"intent":verb,"root":root,"scope":option(&args,"--scope")?,"manifest":if details{Some(captured.kinds.keys().collect::<Vec<_>>())}else{None},"current":current,"baseline_requirements_preserved":preservation,"adoption":"advisory-unverified; no protected adoption channel","contract_sha256":hash(contract_text.as_bytes()),"requested_scope":"all contract obligations; --local never silently drops semantics/runtime","obligations":obligations,"changed_concerns":changed_concerns,"semantic":semantic_results,"fact_sets":fact_sets,"source_paging":segmented_stats,"work":{"parsed":parsed_count,"reused_in_invocation":reused_count,"reused_in_core":reused_count,"reuse_lifetime":if session_transport::enabled(){"explicit foreground Bend core session"}else{"current command private Bend core"},"graph_invalidation":rows.iter().find(|r|r.first().is_some_and(|s|s=="GRAPH_INVALIDATION")).and_then(|r|r.get(1)),"graph_local_changes":rows.iter().filter(|r|r.first().is_some_and(|s|s=="GRAPH_LOCAL")).count(),"graph_invalidated":rows.iter().filter(|r|r.first().is_some_and(|s|s=="GRAPH_DIRTY")).count(),"invalidated_nodes":if details{Some(rows.iter().filter(|r|r.first().is_some_and(|s|s=="GRAPH_DIRTY")).filter_map(|r|r.get(1)).collect::<Vec<_>>())}else{None},"disk_results_admitted":0},"coverage":{"applicable":next.len(),"unresolved":unresolved,"semantic_requests_prepared":requests.len()},"timings_ms":{"capture":capture_ms,"discovery":discovery_ms,"read_plan":plan_ms,"read_decode":read_ms,"bend_parse_fact_evaluate":core_ms,"native_syntax":native_parse_ms,"contract_plan":contract_plan_ms,"revalidate":revalidate_ms,"admission":admission_ms,"summary":summary_ms,"total":started.elapsed().as_millis()},"inputs":{"files":files.len(),"bytes":total,"revalidation_mode":revalidation_mode,"listing_complete":captured.complete,"problems":captured.problems,"issues":captured.issues,"excluded_basenames":captured.exclusions,"observation":"captured bytes with metadata and content revalidation; not a qualified immutable host snapshot"},"cache":"disk entries are advisory comparisons only; exact checks recompute; identical content/grammar facts reuse within the trusted invocation","native":if verb=="verify"{"qualified host runtime binding unavailable; named verifier obligations remain unknown"}else{"not requested"}});
    if verb == "index" {
        // Already written, as the core answered them (`Streamed`).
        report.as_object_mut().ok_or("report object")?.remove("fact_sets");
    }
    if let Some(c) = &content_chunks {
        report["content_chunks"] = content_chunk_report(c, &captured)?;
        report["timings_ms"]["content_chunks"] = json!(c.ms);
    }
    // Agent-facing summary: agents keep every report in context, so `--brief` keeps each
    // obligation's state and, for the rest, only what the next step needs.
    if args.iter().any(|a| a == "--brief") && verb != "index" {
        let obligations = report["obligations"].as_array().ok_or("obligations")?.iter().map(|o| {
            let mut b = json!({"id": o["id"], "state": o["state"]});
            if o["state"] != "verified" {
                for k in ["next_action", "counterexample", "chunk_coverage"] {
                    if !o[k].is_null() {
                        b[k] = o[k].clone();
                    }
                }
            }
            b
        }).collect::<Vec<_>>();
        let problems = report["inputs"]["problems"].as_object().map_or(0, |p| p.len());
        let sample = report["inputs"]["problems"].as_object().map_or(Vec::new(), |p| p.keys().take(10).cloned().collect());
        let recoverable = report["inputs"]["problems"].as_object().map_or(Vec::new(), |p| p.iter()
            .filter(|(_, problem)| problem[1].as_str().is_some_and(|reason| reason.starts_with("ResourcePressure at byte ")))
            .take(10).map(|(path, problem)| json!({"path":path,"reason":problem[1]})).collect());
        report = json!({"schema": "ultragoal/1", "brief": true, "operation": report["operation"], "state": report["state"],
            "current": report["current"], "coverage": report["coverage"], "obligations": obligations,
            "changed_concerns": report["changed_concerns"], "input_problems": {"count": problems, "first": sample, "recoverable": recoverable},
            "details": "rerun without --brief for inputs, facts, timings and full obligation metadata"});
    }
    Ok((report, code))
}

mod context;
mod index;
mod proof;
mod provider;
fn semantic(args: &[String]) -> Result<(Value, i32), String> {
    provider::assess(args)
}

fn rust_syntax(args: &[String]) -> Result<(Value, i32), String> {
    let input = option(args, "--rust")?.ok_or("--rust required")?;
    let bytes = read_adaptive(Path::new(&input))?;
    let start = Instant::now();
    let source = std::str::from_utf8(&bytes).map_err(|_| "invalid UTF-8")?;
    if args.contains(&"--events".to_string()) {
        eprintln!(
            "{}",
            json!({"event":"native-input-captured","bytes":bytes.len()})
        );
    }
    let parsed = native_syntax::rust(source);
    let elapsed = start.elapsed().as_micros();
    let disposition = evaluate(
        "NATIVE\n".to_string() + &row(&[if parsed.is_ok() { "0" } else { "1" }, "complete"]),
    )?;
    let state = disposition.first().ok_or("missing syntax disposition")?;
    if state.len() != 3 {
        return Err("syntax disposition schema".into());
    }
    Ok((
        json!({"schema":"ultragoal/1","state":state[1],"surface":"Rust source syntax only; no type/macro/build/runtime proof","parser":"syn-2.0.117","input_sha256":hash(&bytes),"input_binding":"captured bytes passed directly to parser","parse_us":elapsed,"diagnostic":parsed.err().map(|e|e.to_string()),"compatibility":"temporary mature Rust grammar adapter while Bend structural coverage is qualified"}),
        state[2].parse().map_err(|_| "syntax exit")?,
    ))
}

fn native_syntax(args: &[String], language: &str) -> Result<(Value, i32), String> {
    let input = option(args, &format!("--{language}"))?.ok_or("syntax input required")?;
    let bytes = read_adaptive(Path::new(&input))?;
    std::str::from_utf8(&bytes).map_err(|_| "invalid UTF-8")?;
    // Tool locations come from an optional `--tools FILE` JSON object (python, node,
    // typescript); each observation hashes the tool and module it ran.
    let tools = option(args, "--tools")?
        .map(|p| read(Path::new(&p), 64 * 1024).and_then(|b| provider::decode(&b)))
        .transpose()?
        .unwrap_or(Value::Null);
    let tool_path = |key: &str, default: &str| tools[key].as_str().unwrap_or(default).to_string();
    let (tool, script, extra) = if language == "python" {
        (
            tool_path("python", "/usr/bin/python3"),
            "import ast,json,sys\ns=sys.stdin.buffer.read().decode('utf-8')\ntry:\n ast.parse(s,filename='<captured-input>')\n print(json.dumps({'parser':sys.version,'diagnostics':[]}))\nexcept SyntaxError as e:\n print(json.dumps({'parser':sys.version,'diagnostics':[{'message':str(e),'line':e.lineno,'offset':e.offset}]}))\n sys.exit(1)",
            None,
        )
    } else {
        (
            tool_path("node", "/opt/homebrew/bin/node"),
            "const ts=require(process.argv[1]);let s='';process.stdin.setEncoding('utf8');process.stdin.on('data',x=>s+=x);process.stdin.on('end',()=>{const f=ts.createSourceFile('snapshot.ts',s,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);const d=f.parseDiagnostics.map(x=>({code:x.code,start:x.start,length:x.length,message:ts.flattenDiagnosticMessageText(x.messageText,' ')}));console.log(JSON.stringify({parser:ts.version,diagnostics:d}));process.exitCode=d.length?1:0;});",
            Some(tool_path("typescript", "/opt/homebrew/lib/node_modules/typescript/lib/typescript.js")),
        )
    };
    let tool = fs::canonicalize(&tool)
        .map_err(|_| format!("{language} native parser unavailable at {tool}"))?;
    let extra = extra.map(|module| {
        fs::canonicalize(&module)
            .map_err(|_| format!("{language} parser module unavailable at {module}"))
    }).transpose()?;
    let mut command = Command::new(&tool);
    command.env_clear().env("PATH", "/usr/bin:/bin");
    if language == "python" {
        command.args(["-I", "-S", "-B", "-c", script]);
    } else {
        command.args(["--input-type=commonjs", "-e", script]);
    }
    if let Some(module) = &extra {
        command.arg(module);
    }
    native_observation::Pending::issue(
        if language=="python" {"python-ast"} else {"typescript-parse"},
        command,Path::new(&input),bytes,extra.as_deref(),
    )?.execute()
}

fn native_probe(args: &[String]) -> Result<(Value, i32), String> {
    let input = option(args, "--stdin-c")?.ok_or("--stdin-c required")?;
    let bytes = read_adaptive(Path::new(&input))?;
    // Complete single-translation-unit envelope excludes directives and headers.
    // This is conservative (even a # in a comment is unsupported), never silently
    // elevated to whole-project runtime proof.
    let source = std::str::from_utf8(&bytes).map_err(|_| "source UTF-8")?;
    let admission = if bytes.len() > MAX / 4 {
        segmented::native_plan_captured(Path::new(&input), &bytes)?
    } else {
        evaluate("NATIVE_PLAN\n".to_string() + &row(&[source]))?
    };
    let admitted = admission.first().ok_or("native admission missing")?;
    if admitted.len() != 3 || admitted[0] != "ADMITTED" {
        return Ok((
            json!({"schema":"ultragoal/1","state":"unknown","admission":admission}),
            2,
        ));
    }
    let mut locate = Command::new("/usr/bin/xcrun");
    locate.args(["--find", "clang"]);
    let located = run(locate, Vec::new(), Duration::from_secs(3))?;
    if located.code != Some(0) {
        return Err("clang unavailable".into());
    }
    let tool = String::from_utf8(located.out)
        .map_err(|_| "compiler path UTF-8")?
        .trim()
        .to_string();
    let argv = [
        "--no-default-config",
        "-x",
        "c",
        "-std=c11",
        "-nostdinc",
        "-undef",
        "-fsyntax-only",
        "-",
    ];
    let mut c = Command::new(&tool);
    c.env_clear().env("PATH", "/usr/bin:/bin").args(argv);
    native_observation::Pending::issue("c11-stdin",c,Path::new(&input),bytes,None)?.execute()
}

#[cfg(test)]
mod error_code_tests {
    use super::{error_code, Failure};

    #[test]
    fn typed_consequential_codes_survive_diagnostic_rewording() {
        for diagnostic in ["source changed since listing", "different words with no key phrase"] {
            assert_eq!(Failure::InputChanged(diagnostic.into()).code(), "input_changed");
            assert_eq!(Failure::Cancelled(diagnostic.into()).code(), "cancelled");
        }
        assert_eq!(Failure::ResourcePressure { path: "x".into(), offset: 4096 }.code(), "resource_bound");
        assert_eq!(Failure::from_segment(super::segmented::EvalError::SourceChanged("x".into())).code(), "input_changed");
        assert_eq!(Failure::from_segment(super::segmented::EvalError::Source(super::fs_adapter::StreamError::Changed)).code(), "input_changed");
        assert_eq!(Failure::from_segment(super::segmented::EvalError::Source(super::fs_adapter::StreamError::Cancelled)).code(), "cancelled");
        assert_eq!(Failure::from_segment(super::segmented::EvalError::Source(super::fs_adapter::StreamError::Pressure { offset: 17 })).code(), "resource_bound");
    }

    #[test]
    fn failures_map_to_stable_codes() {
        for (message, code) in [
            ("unknown option --wording", "invalid_arguments"),
            ("--request required", "invalid_arguments"),
            ("--local conflicts with --semantic", "invalid_arguments"),
            ("missing --query value", "invalid_arguments"),
            ("semantic budget refused: --semantic-deadline-ms 500-15000, --semantic-concurrency 1-4", "invalid_arguments"),
            ("session lifetime refused: --lifetime-seconds 30-7200, --idle-seconds 5-1800", "invalid_arguments"),
            ("explicit session handle unavailable; start a new session or deliberately run cold: No such file or directory (os error 2)", "session_unavailable"),
            ("explicit session unavailable; run cold deliberately or start a new session: Connection refused (os error 61)", "session_unavailable"),
            ("session socket refused: Operation not permitted (os error 1); the host must allow local Unix sockets; paths /bound/limit/core.sock", "session_unavailable"),
            ("advice context excluded by disclosure policy", "disclosure_refused"),
            ("credential lookup unavailable", "credential_refused"),
            ("content chunk request 15: core request deadline exceeded; paths a/limit.py .. b.py", "deadline_exceeded"),
            ("evaluation (PLAN): core request deadline exceeded", "deadline_exceeded"),
            ("core request cancelled", "cancelled"),
            ("owned core process-group termination unavailable", "core_unavailable"),
            ("index request 3: core writer closed; core diagnosis: exit status: 1; stderr: limit exceeded; paths a .. b", "core_unavailable"),
            ("core closed before complete response; owned core process-group termination unavailable; core diagnosis: signal: 9 (SIGKILL); stderr: none", "core_unavailable"),
            ("session socket path /x/core.sock exceeds the 103-byte sun_path limit, and the fallback /t/ug-1.sock is too long", "invalid_arguments"),
            ("wire frame exceeds 16 MiB", "resource_bound"),
            ("input changed during read", "input_changed"),
            ("compiled ug-core must sit beside ultragoal; run build.py", "core_unavailable"),
            ("No such file or directory (os error 2)", "input_unavailable"),
            ("invalid Bend result envelope", "invalid_input"),
            ("something new", "operation_failed"),
        ] {
            assert_eq!(error_code(message), code, "{message}");
        }
    }
}
