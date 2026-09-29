//! Private framed transport to one compiled core per CLI invocation.
//! No rule selection, graph planning, authorization or semantic policy here.
use super::*;
use std::io::{BufRead, BufReader};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::process::Child;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    mpsc::{self, Receiver, RecvTimeoutError, Sender},
};
use std::thread::JoinHandle;

const SESSION_BYTES: usize = 512 * 1024 * 1024;
/// Retained operations may legitimately carry more than the ordinary core's
/// cumulative traffic budget. Recheck live headroom on each page; this is an
/// operational pressure boundary, rather than a source-size ceiling.
fn retained_budget() -> usize {
    resources::current().map(|h| usize::try_from(h.work_bytes() / 8).unwrap_or(usize::MAX))
        .unwrap_or(SESSION_BYTES)
}
/// Stall bound of one request on a dedicated core. Each such request is bounded by
/// bytes and rows, so total time grows with the number of requests, not a phase cap.
pub const REQUEST_STALL: Duration = Duration::from_secs(30);

fn late(deadline: Instant, cancelled: Option<&AtomicBool>) -> Result<(), String> {
    if is_interrupted() {
        return Err("core request cancelled".into());
    }
    if cancelled.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
        return Err("session client disconnected".into());
    }
    if Instant::now() >= deadline {
        return Err("core request deadline exceeded".into());
    }
    Ok(())
}

enum WriteMessage {
    Bytes(Vec<u8>),
    Close,
}
enum Message {
    Chunk {
        sequence: u64,
        done: bool,
        bytes: Vec<u8>,
    },
    Error(String),
    Eof,
}

fn header(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, String> {
    let mut line = Vec::new();
    for _ in 0..128 {
        let mut byte = [0];
        match reader.read(&mut byte) {
            Ok(0) if line.is_empty() => return Ok(None),
            Ok(0) => return Err("truncated core response header".into()),
            Ok(_) if byte[0] == b'\n' => return Ok(Some(line)),
            Ok(_) => line.push(byte[0]),
            Err(e) => return Err(format!("core response header: {e}")),
        }
    }
    Err("oversized core response header".into())
}

fn number(text: &str) -> Result<u64, String> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err("noncanonical core response number".into());
    }
    let n = text
        .parse::<u64>()
        .map_err(|_| "core response number overflow")?;
    if n.to_string() != text {
        return Err("noncanonical core response number".into());
    }
    Ok(n)
}

fn reader(
    stdout: impl Read,
    tx: mpsc::SyncSender<Message>,
    expected: Arc<AtomicU64>,
    budget: Arc<AtomicUsize>,
) {
    let mut input = BufReader::new(stdout);
    let mut run = || -> Result<(), String> {
        loop {
            let Some(line) = header(&mut input)? else {
                let _ = tx.send(Message::Eof);
                return Ok(());
            };
            let text = std::str::from_utf8(&line).map_err(|_| "non-UTF8 core header")?;
            let fields = text.split('\t').collect::<Vec<_>>();
            if fields.len() != 4 || fields[0] != "S1" || !matches!(fields[2], "more" | "done") {
                return Err("invalid core response envelope".into());
            }
            let sequence = number(fields[1])?;
            if sequence != expected.load(Ordering::SeqCst) {
                return Err("core response sequence mismatch".into());
            }
            let size = usize::try_from(number(fields[3])?).map_err(|_| "core length overflow")?;
            budget
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                    left.checked_sub(size)
                })
                .map_err(|_| "core response exceeds operation byte bound")?;
            let mut bytes = vec![0; size];
            input
                .read_exact(&mut bytes)
                .map_err(|e| format!("truncated core response: {e}"))?;
            if tx
                .send(Message::Chunk {
                    sequence,
                    done: fields[2] == "done",
                    bytes,
                })
                .is_err()
            {
                return Ok(());
            }
        }
    };
    if let Err(e) = run() {
        let _ = tx.send(Message::Error(e));
    }
}

/// The core reads each frame in at most 4,096 reads of up to 64 KiB. A pipe holds 16 to
/// 64 KiB, so on a loaded machine the core wakes to small reads and a frame of a few
/// MiB passes that bound. A socket pair with a 4 MiB buffer lets the writer stay far
/// ahead of the core, so its reads stay full.
const STDIN_BUFFER: libc::c_int = 4 * 1024 * 1024;

fn input_channel() -> std::io::Result<(UnixStream, UnixStream)> {
    let (ours, core) = UnixStream::pair()?;
    for (end, option) in [(&ours, libc::SO_SNDBUF), (&core, libc::SO_RCVBUF)] {
        let size = STDIN_BUFFER;
        let set = unsafe {
            libc::setsockopt(
                end.as_raw_fd(),
                libc::SOL_SOCKET,
                option,
                (&size as *const libc::c_int).cast(),
                std::mem::size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        if set != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok((ours, core))
}

fn writer(mut stdin: UnixStream, rx: Receiver<WriteMessage>, failed: Arc<AtomicBool>, error: Arc<Mutex<Option<String>>>) {
    while let Ok(message) = rx.recv() {
        let (bytes, close) = match message {
            WriteMessage::Bytes(bytes) => (bytes, false),
            WriteMessage::Close => (b"0\n".to_vec(), true),
        };
        if let Err(e) = stdin.write_all(&bytes).and_then(|_| stdin.flush()) {
            failed.store(true, Ordering::SeqCst);
            if let Ok(mut slot) = error.lock() {
                *slot = Some(e.to_string());
            }
            break;
        }
        if close {
            break;
        }
    }
}

pub struct Session {
    child: Child,
    input: Option<Sender<WriteMessage>>,
    output: Option<Receiver<Message>>,
    threads: Vec<JoinHandle<()>>,
    expected: Arc<AtomicU64>,
    budget: Arc<AtomicUsize>,
    failed: Arc<AtomicBool>,
    next: u64,
    input_bytes: usize,
    output_bytes: usize,
    retained: bool,
    stopped: bool,
    /// Sequence and lines of the last evaluation body this core retained.
    base: Option<(u64, Vec<String>)>,
    evaluations: u64,
    /// The last STDERR_TAIL bytes the core wrote, and its exit status once reaped.
    stderr: Arc<Mutex<Vec<u8>>>,
    status: Option<std::process::ExitStatus>,
    write_error: Arc<Mutex<Option<String>>>,
}

const STDERR_TAIL: usize = 4096;
const CORE_REQUESTS: u64 = 4096;

impl Session {
    fn start(path: &Path) -> Result<Self, String> {
        let mut command = Command::new(path);
        command
            .args(["--threads", "8", "--gpu", "off", "--", "--session"])
            .process_group(0);
        let (stdin, core_stdin) = input_channel().map_err(|e| format!("core session start: {e}"))?;
        let mut child = command
            .stdin(Stdio::from(std::os::fd::OwnedFd::from(core_stdin)))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("core session start: {e}"))?;
        let stdout = child.stdout.take().ok_or("core stdout unavailable")?;
        let mut stderr = child.stderr.take().ok_or("core stderr unavailable")?;
        let expected = Arc::new(AtomicU64::new(0));
        let budget = Arc::new(AtomicUsize::new(MAX));
        let failed = Arc::new(AtomicBool::new(false));
        let (input, writes) = mpsc::channel();
        let (responses, output) = mpsc::sync_channel(1);
        let tail = Arc::new(Mutex::new(Vec::new()));
        let write_error = Arc::new(Mutex::new(None));
        let (e, b, f, ef, t, w) = (
            expected.clone(),
            budget.clone(),
            failed.clone(),
            failed.clone(),
            tail.clone(),
            write_error.clone(),
        );
        let threads = vec![
            std::thread::spawn(move || writer(stdin, writes, f, w)),
            std::thread::spawn(move || reader(stdout, responses, e, b)),
            std::thread::spawn(move || {
                let mut buffer = [0; 8192];
                let mut total = 0usize;
                loop {
                    match stderr.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            total = total.saturating_add(n);
                            if total > MAX {
                                ef.store(true, Ordering::SeqCst);
                            }
                            if let Ok(mut t) = t.lock() {
                                t.extend_from_slice(&buffer[..n]);
                                let cut = t.len().saturating_sub(STDERR_TAIL);
                                t.drain(..cut);
                            }
                        }
                        Err(_) => {
                            ef.store(true, Ordering::SeqCst);
                            break;
                        }
                    }
                }
            }),
        ];
        Ok(Self {
            child,
            input: Some(input),
            output: Some(output),
            threads,
            expected,
            budget,
            failed,
            next: 0,
            input_bytes: 0,
            output_bytes: 0,
            retained: false,
            stopped: false,
            base: None,
            evaluations: 0,
            stderr: tail,
            status: None,
            write_error,
        })
    }

    fn stop(&mut self) -> Result<(), String> {
        if !self.stopped {
            if self.child.try_wait().map_err(|e| e.to_string())?.is_none() {
                let result =
                    unsafe { libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL) };
                // macOS answers EPERM, not ESRCH, while the group's only member is exiting
                // or a zombie; such a core is reaped within moments.
                if result != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                    let limit = Instant::now() + Duration::from_secs(2);
                    while self.child.try_wait().map_err(|e| e.to_string())?.is_none() {
                        if Instant::now() >= limit {
                            return Err("owned core process-group termination unavailable".into());
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
            }
            self.status = Some(self.child.wait().map_err(|e| e.to_string())?);
            self.stopped = true;
        }
        self.input.take();
        self.output.take(); // Release any blocked bounded-channel sender before joins.
        for thread in self.threads.drain(..) {
            thread.join().map_err(|_| "core transport thread stopped")?;
        }
        Ok(())
    }

    /// Evaluation bodies after the first are sent as a line delta against the
    /// body this core retained. The core rebuilds the exact lines from its
    /// retained values or answers DELTA_UNAVAILABLE, and the full body is sent.
    fn exchange(
        &mut self,
        frames: Vec<String>,
        index: bool,
        deadline: Instant,
        limit: usize,
    ) -> Result<Vec<u8>, String> {
        self.exchange_with_cancel(frames, index, deadline, limit, None)
    }

    fn exchange_with_cancel(
        &mut self,
        frames: Vec<String>,
        index: bool,
        deadline: Instant,
        limit: usize,
        cancelled: Option<&AtomicBool>,
    ) -> Result<Vec<u8>, String> {
        let evaluation = !index && frames.len() == 1 && frames[0].starts_with("UG\t");
        if !evaluation {
            return self.exchange_raw(frames, index, deadline, limit, cancelled).map(|(_, bytes)| bytes);
        }
        let lines: Vec<String> = frames[0].split('\n').map(str::to_string).collect();
        self.evaluations += 1;
        if let Some((seq, old)) = self.base.take() {
            let body = delta_body(seq, &old, &lines);
            if body.len() < frames[0].len() / 2 {
                let (sequence, bytes) = self.exchange_raw(vec![body], false, deadline, limit, cancelled)?;
                if bytes != DELTA_UNAVAILABLE {
                    self.base = Some((sequence, lines));
                    return Ok(bytes);
                }
            }
        }
        let (sequence, bytes) = self.exchange_raw(frames, false, deadline, limit, cancelled)?;
        self.base = Some((sequence, lines));
        Ok(bytes)
    }

    fn exchange_raw(
        &mut self,
        frames: Vec<String>,
        index: bool,
        deadline: Instant,
        limit: usize,
        cancelled: Option<&AtomicBool>,
    ) -> Result<(u64, Vec<u8>), String> {
        if self.stopped
            || self.next >= CORE_REQUESTS
            || frames.is_empty()
            || frames.len() > 128
            || limit > REPORT_MAX
        {
            return Err("core session unavailable or transport work bound".into());
        }
        let sequence = self.next;
        let mut wire = Vec::new();
        let control = if index {
            format!("S1\t{sequence}\tINDEX\t{}\n", frames.len())
        } else {
            if frames.len() != 1 {
                return Err("single request frame required".into());
            }
            format!("S1\t{sequence}\tONE\n{}", frames[0])
        };
        append(&mut wire, control.as_bytes())?;
        if index {
            for frame in frames {
                append(&mut wire, frame.as_bytes())?;
            }
        }
        let total_input = self
            .input_bytes
            .checked_add(wire.len())
            .ok_or("core session input overflow")?;
        let capacity = if self.retained { retained_budget() } else { SESSION_BYTES };
        if total_input > capacity {
            return Err("core cumulative input byte bound".into());
        }
        late(deadline, cancelled)?;
        self.expected.store(sequence, Ordering::SeqCst);
        let remaining_output = capacity
            .checked_sub(self.output_bytes)
            .ok_or("core cumulative output overflow")?;
        self.budget
            .store(limit.min(remaining_output), Ordering::SeqCst);
        let sent = self
            .input
            .as_ref()
            .ok_or("core input closed")?
            .send(WriteMessage::Bytes(wire));
        if sent.is_err() {
            return Err(self.failure("core writer closed".into()));
        }
        self.next += 1;
        self.input_bytes = total_input;
        let result = (|| {
            let mut output = Vec::new();
            let mut chunks = 0usize;
            loop {
                late(deadline, cancelled)?;
                if self.failed.load(Ordering::SeqCst) {
                    return Err("core transport write/stderr failure".into());
                }
                match self
                    .output
                    .as_ref()
                    .ok_or("core output closed")?
                    .recv_timeout(Duration::from_millis(10))
                {
                    Ok(Message::Chunk {
                        sequence: observed,
                        done,
                        bytes,
                    }) => {
                        late(deadline, cancelled)?;
                        chunks += 1;
                        if observed != sequence
                            || chunks > 128
                            || (!index && !done)
                            || output.len().saturating_add(bytes.len()) > limit
                        {
                            return Err("invalid core response sequence/chunk/byte bound".into());
                        }
                        output.extend(bytes);
                        if done {
                            return Ok(output);
                        }
                    }
                    Ok(Message::Error(e)) => return Err(e),
                    Ok(Message::Eof) => return Err("core closed before complete response".into()),
                    Err(RecvTimeoutError::Disconnected) => {
                        return Err("core response channel closed".into());
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                }
            }
        })();
        let bytes = result.map_err(|e| self.failure(e))?;
        self.output_bytes = self
            .output_bytes
            .checked_add(bytes.len())
            .ok_or("core cumulative output overflow")?;
        Ok((sequence, bytes))
    }

    /// Stops the core after a failed request and reports the failure with how the
    /// core ended and the end of its stderr.
    fn failure(&mut self, error: String) -> String {
        let stopped = self.stop().err().map_or(String::new(), |e| format!("; {e}"));
        let tail = self.stderr.lock().map(|t| String::from_utf8_lossy(&t).trim().to_string()).unwrap_or_default();
        let status = self.status.map_or("not reaped".into(), |s| s.to_string());
        let write = self.write_error.lock().ok().and_then(|w| w.clone()).map_or(String::new(), |w| format!("; stdin write: {w}"));
        format!("{error}{stopped}; core diagnosis: {status}{write}; stderr: {}", if tail.is_empty() { "none" } else { &tail })
    }

    /// Whether this core can take `input` more wire bytes and an answer of up to
    /// `limit` bytes within `budget` cumulative bytes per direction.
    fn fits(&self, input: usize, limit: usize, budget: usize) -> bool {
        !self.stopped
            && self.next < CORE_REQUESTS
            && self.input_bytes.saturating_add(input) <= budget
            && self.output_bytes.saturating_add(limit) <= budget
    }

    fn finish(&mut self) -> Result<(), String> {
        if self.stopped {
            return Ok(());
        }
        let closed = self.input.as_ref().is_some_and(|input| input.send(WriteMessage::Close).is_ok());
        if !closed {
            return Err(self.failure("core writer closed before close".into()));
        }
        self.input.take();
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut error = None;
        let mut exited = false;
        let mut eof = false;
        loop {
            if let Some(status) = self.child.try_wait().map_err(|e| e.to_string())? {
                if !status.success() {
                    error = Some("core did not terminate successfully".to_string());
                }
                exited = true;
            }
            if exited && eof {
                break;
            }
            if Instant::now() >= deadline || is_interrupted() {
                error = Some("core termination deadline/cancellation".into());
                break;
            }
            match self
                .output
                .as_ref()
                .ok_or("core output closed")?
                .recv_timeout(Duration::from_millis(10))
            {
                Ok(Message::Chunk { .. }) => {
                    error = Some("unexpected response after final request".into())
                }
                Ok(Message::Error(e)) => error = Some(e),
                Ok(Message::Eof) => eof = true,
                Err(RecvTimeoutError::Disconnected) if eof => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(RecvTimeoutError::Disconnected) => {
                    error = Some("core response channel closed without EOF".into());
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
        if self.failed.load(Ordering::SeqCst) {
            error = Some("core transport failure during close".into());
        }
        if let Err(e) = self.stop() {
            error.get_or_insert(e);
        }
        if self.failed.load(Ordering::SeqCst) {
            error = Some("core transport failure during close".into());
        }
        error.map_or(Ok(()), |e| Err(self.failure(e)))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

const DELTA_UNAVAILABLE: &[u8] = b"ERROR\tdelta base unavailable\n";

enum Edit<'a> {
    Keep,
    Skip,
    Literal(&'a str),
}

/// Monotone line edit script turning `old` into `new`: every old line is kept or
/// skipped in order and unmatched new lines are literals. A line is kept only
/// where it equals the old line, so the script is exact whatever it matches.
/// Each new line keeps the first equal old line at most 64 lines ahead; comparing
/// in place avoids hashing every line of a large frame.
fn edits<'a>(old: &[String], new: &'a [String]) -> Vec<Edit<'a>> {
    let mut out = Vec::with_capacity(old.len() + 8);
    let mut j = 0usize;
    for line in new {
        match (j..old.len().min(j + 65)).find(|&p| old[p] == *line) {
            Some(p) => {
                out.extend((j..p).map(|_| Edit::Skip));
                out.push(Edit::Keep);
                j = p + 1;
            }
            None => out.push(Edit::Literal(line)),
        }
    }
    out.extend((j..old.len()).map(|_| Edit::Skip));
    out
}

fn delta_body(seq: u64, old: &[String], new: &[String]) -> String {
    let script = edits(old, new);
    let mut body = format!("UG_DELTA\t{seq}\n");
    let mut i = 0usize;
    while i < script.len() {
        match script[i] {
            Edit::Literal(line) => {
                body += "L\t";
                body += line;
                body.push('\n');
                i += 1;
            }
            Edit::Keep | Edit::Skip => {
                let keep = matches!(script[i], Edit::Keep);
                let run = script[i..]
                    .iter()
                    .take_while(|e| matches!(e, Edit::Keep) == keep && !matches!(e, Edit::Literal(_)))
                    .count();
                body += &format!("{}\t{run}\n", if keep { "K" } else { "D" });
                i += run;
            }
        }
    }
    body
}

fn append(wire: &mut Vec<u8>, frame: &[u8]) -> Result<(), String> {
    if frame.is_empty() || frame.len() > MAX {
        return Err("core frame exceeds16MiB or is empty".into());
    }
    if wire.len().saturating_add(frame.len()).saturating_add(16) > 512 * 1024 * 1024 {
        return Err("core session input byte bound".into());
    }
    wire.extend_from_slice(format!("{}\n", frame.len()).as_bytes());
    wire.extend_from_slice(frame);
    Ok(())
}

static SESSION: OnceLock<Mutex<Option<Session>>> = OnceLock::new();

/// Evaluations served by one owner core before a primed replacement is prepared.
/// The runtime's LIFO free lists scatter allocations as a core ages, so the same
/// request costs more cycles later; a fresh core primed with the same frame retains
/// the same state.
const ROTATE_EVERY: u64 = 10;
static ROTATION: AtomicBool = AtomicBool::new(false);
type Standby = (Vec<String>, JoinHandle<Result<Session, String>>);
static STANDBY: OnceLock<Mutex<Option<Standby>>> = OnceLock::new();

pub fn enable_rotation() {
    ROTATION.store(true, Ordering::SeqCst);
}

/// A fresh core evaluates the same full frame the active core is about to evaluate,
/// so both retain the state of the same last frame.
fn prime(frame: &str) -> Result<Standby, String> {
    let lines = frame.split('\n').map(str::to_string).collect();
    let (frame, path) = (frame.to_string(), core()?);
    Ok((
        lines,
        std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(30);
            let mut fresh = Session::start(&path)?;
            fresh.exchange(vec!["EXCLUSIONS\n".into()], false, deadline, MAX)?;
            fresh.exchange(vec![frame], false, deadline, MAX)?;
            Ok(fresh)
        }),
    ))
}

/// Between requests: swap in a finished replacement primed with exactly the
/// active core's current base frame; a stale one is dropped.
pub fn maintain() -> Result<(), String> {
    let Some(cell) = SESSION.get() else {
        return Ok(());
    };
    let Ok(mut guard) = cell.try_lock() else {
        return Ok(());
    };
    let Some(active) = guard.as_mut() else {
        return Ok(());
    };
    let mut standby = STANDBY
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "standby core lock poisoned")?;
    if standby.as_ref().is_some_and(|(_, handle)| handle.is_finished())
        && let Some((primed, handle)) = standby.take()
    {
        let fresh = handle.join().map_err(|_| "standby core thread panicked")??;
        if active.base.as_ref().map(|(_, lines)| lines) == Some(&primed) {
            std::mem::replace(active, fresh).finish()?;
        }
    }
    Ok(())
}

pub fn request(frames:Vec<String>,index:bool,deadline:Instant,limit:usize)->Result<Vec<u8>,String>{
    if session_transport::enabled(){session_transport::request(frames,index,deadline,limit)}
    else{request_local(frames,index,deadline,limit)}
}
pub fn finish()->Result<(),String>{
    if session_transport::enabled(){session_transport::finish()}else{finish_local()}
}
/// Independent multi-frame requests on dedicated cores, each under its own stall
/// deadline. A core is replaced before a request would pass its cumulative byte
/// bound, so the bound still holds per process while a lane carries any volume.
pub struct Lane {
    path: PathBuf,
    phase: &'static str,
    budget: usize,
    stall: Duration,
    session: Option<Session>,
    requests: usize,
    cores: usize,
}

impl Lane {
    pub fn new(phase: &'static str) -> Result<Self, String> {
        Ok(Self::with(core()?, phase, SESSION_BYTES, REQUEST_STALL))
    }

    fn with(path: PathBuf, phase: &'static str, budget: usize, stall: Duration) -> Self {
        Self { path, phase, budget, stall, session: None, requests: 0, cores: 0 }
    }

    pub fn request(&mut self, frames: Vec<String>, limit: usize) -> Result<Vec<u8>, String> {
        let index = self.requests;
        self.requests += 1;
        let wire = 32 + frames.iter().map(|f| f.len() + f.len().to_string().len() + 1).sum::<usize>();
        let result = (|| {
            if let Some(mut old) = self.session.take_if(|s| !s.fits(wire, limit, self.budget)) {
                old.finish()?;
            }
            if self.session.is_none() {
                self.session = Some(Session::start(&self.path)?);
                self.cores += 1;
            }
            self.session
                .as_mut()
                .ok_or("lane core absent")?
                .exchange(frames, true, Instant::now() + self.stall, limit)
        })();
        result.map_err(|e| format!("{} request {index}: {e}", self.phase))
    }

    /// Ordered one-frame commands that keep Bend's pending logical-file state.
    /// A lane cannot rotate midway through a source; the caller must report a
    /// recoverable incomplete operation before its physical traffic budget.
    pub fn request_one(&mut self, frame: String, limit: usize) -> Result<Vec<u8>, String> {
        let index = self.requests;
        self.requests += 1;
        let wire = 32 + frame.len() + frame.len().to_string().len();
        if self.session.is_none() {
            self.session = Some(Session::start(&self.path)?);
            self.cores += 1;
        }
        let session = self.session.as_mut().ok_or("segment lane absent")?;
        session.retained = true;
        if !session.fits(wire, limit, retained_budget()) {
            return Err(format!("{} request {index}: retained operation exceeds current core traffic headroom; retry from revalidated input", self.phase));
        }
        session.exchange(vec![frame], false, Instant::now() + self.stall, limit)
            .map_err(|e| format!("{} request {index}: {e}", self.phase))
    }

    /// Closes the current core and returns how many cores the lane used.
    pub fn finish(mut self) -> Result<usize, String> {
        if let Some(mut session) = self.session.take() {
            session.finish()?;
        }
        Ok(self.cores)
    }
}
pub fn observed_pid()->Option<u32>{SESSION.get()?.lock().ok()?.as_ref().map(|s|s.child.id())}

pub fn request_local(
    frames: Vec<String>,
    index: bool,
    deadline: Instant,
    limit: usize,
) -> Result<Vec<u8>, String> {
    request_local_with_cancel(frames, index, deadline, limit, None)
}

pub fn request_local_with_cancel(
    frames: Vec<String>,
    index: bool,
    deadline: Instant,
    limit: usize,
    cancelled: Option<&AtomicBool>,
) -> Result<Vec<u8>, String> {
    let cell = SESSION.get_or_init(|| Mutex::new(None));
    let mut guard = loop {
        late(deadline, cancelled)?;
        match cell.try_lock() {
            Ok(guard) => break guard,
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err("core session lock poisoned".into());
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                std::thread::sleep(Duration::from_millis(2))
            }
        }
    };
    if guard.is_none() {
        *guard = Some(Session::start(&core()?)?);
    }
    let session = guard.as_mut().ok_or("core session absent")?;
    if ROTATION.load(Ordering::SeqCst)
        && session.evaluations + 1 >= ROTATE_EVERY
        && !index
        && frames.len() == 1
        && frames[0].starts_with("UG\t")
    {
        let mut standby = STANDBY
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| "standby core lock poisoned")?;
        if standby.is_none() {
            *standby = Some(prime(&frames[0])?);
        }
    }
    let result = session.exchange_with_cancel(frames, index, deadline, limit, cancelled);
    if cancelled.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
        if let Some(mut abandoned) = guard.take() {
            abandoned.stop()?;
        }
        return Err("session client disconnected".into());
    }
    result
}

pub fn finish_local() -> Result<(), String> {
    if let Some(standby) = STANDBY.get() {
        let pending = standby.lock().map_err(|_| "standby core lock poisoned")?.take();
        if let Some((_, handle)) = pending {
            drop(handle.join().map_err(|_| "standby core thread panicked")?);
        }
    }
    let Some(cell) = SESSION.get() else {
        return Ok(());
    };
    let mut guard = cell.lock().map_err(|_| "core session lock poisoned")?;
    if let Some(mut session) = guard.take() {
        session.finish()?;
    }
    Ok(())
}

/// Retire a request whose client left after the core answered but before the
/// response was published. Its retained state cannot be assumed observed.
pub fn abandon_local() -> Result<(), String> {
    let Some(cell) = SESSION.get() else { return Ok(()); };
    let mut guard = cell.lock().map_err(|_| "core session lock poisoned")?;
    if let Some(mut session) = guard.take() { session.stop()?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    fn lines(text: &str) -> Vec<String> {
        text.split('\n').map(str::to_string).collect()
    }

    fn rebuilt(old: &[String], body: &str) -> Vec<String> {
        let mut base = old.iter();
        let mut out = Vec::new();
        for op in body.split('\n').skip(1).filter(|l| !l.is_empty()) {
            let (kind, rest) = op.split_once('\t').unwrap();
            match kind {
                "K" => out.extend((0..rest.parse::<usize>().unwrap()).map(|_| base.next().unwrap().clone())),
                "D" => (0..rest.parse::<usize>().unwrap()).for_each(|_| { base.next().unwrap(); }),
                "L" => out.push(rest.to_string()),
                _ => panic!("unknown op"),
            }
        }
        assert!(base.next().is_none(), "base consumed exactly");
        out
    }

    #[test]
    fn delta_script_reconstructs_exact_lines() {
        let old = lines("UG\t1\nO\ta\nOP\t1\trunning\nF\ta.py\tunread\t\nF\tb.py\tunread\t\n\nF\tc\td\te\n");
        let cases = [
            "UG\t1\nO\ta\nOP\t2\trunning\nF\ta.py\tunread\t\nF\tb.py\tunread\t\n\nF\tc\td\te\n",
            "UG\t1\nO\ta\nOP\t2\trunning\nF\tb.py\tunread\t\n\nF\tc\td\te\n",
            "UG\t1\nO\tb\nO\ta\nOP\t2\trunning\nF\ta.py\tunread\t\nF\tnew.py\tx\ty\t\n",
            "",
            "F\tc\td\te\nUG\t1\n",
        ];
        for new in cases {
            let new = lines(new);
            let body = delta_body(7, &old, &new);
            assert!(body.starts_with("UG_DELTA\t7\n"));
            assert_eq!(rebuilt(&old, &body), new);
        }
    }

    use super::*;
    use std::io::Cursor;
    use std::os::unix::fs::PermissionsExt;

    fn read_message(bytes: &[u8], limit: usize) -> Message {
        let (tx, rx) = mpsc::sync_channel(2);
        reader(
            Cursor::new(bytes),
            tx,
            Arc::new(AtomicU64::new(0)),
            Arc::new(AtomicUsize::new(limit)),
        );
        rx.recv().unwrap()
    }

    #[test]
    fn physical_response_envelope_rejects_before_oversized_allocation() {
        for input in [
            b"S1\t1\tdone\t1\nx".as_slice(),
            b"S1\t00\tdone\t1\nx",
            b"S1\t0\tdone\t999999999\n",
            b"S1\t0\tdone\t2\nx",
            b"S1\t0\tunknown\t0\n",
            b"S1\t0\tdone\t+1\nx",
            b"S1\t0\tdone\t18446744073709551616\n",
            b"S1\t0\tdone\t1",
        ] {
            assert!(matches!(read_message(input, 8), Message::Error(_)));
        }
        assert!(
            matches!(read_message(b"S1\t0\tdone\t3\nx\0y", 3), Message::Chunk { done: true, bytes, .. } if bytes == b"x\0y")
        );
        assert!(header(&mut Cursor::new(vec![b'x'; 129])).is_err());
    }

    fn mock(body: &str) -> (PathBuf, PathBuf) {
        let root = env::temp_dir().join(format!("ug-core-session-{}", op_id().unwrap()));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let executable = root.join("core");
        fs::write(&executable, format!("#!/usr/bin/python3\n{body}\n")).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        (root, executable)
    }

    const ECHO: &str = r#"import sys,os
def frame():
 line=sys.stdin.buffer.readline()
 if not line: raise RuntimeError("missing terminator")
 n=int(line)
 if not n: return None
 data=sys.stdin.buffer.read(n)
 if len(data)!=n: raise RuntimeError("truncated frame")
 return data
while True:
 data=frame()
 if data is None:
  assert sys.stdin.buffer.read()==b""
  break
 header,_,payload=data.partition(b"\n")
 fields=header.split(b"\t")
 seq=fields[1]
 if fields[2]==b"INDEX":
  payload=b"".join(frame() for _ in range(int(fields[3])))
 output=str(os.getpid()).encode()+b":"+payload
 sys.stdout.buffer.write(b"S1\t"+seq+b"\tdone\t"+str(len(output)).encode()+b"\n"+output)
 sys.stdout.buffer.flush()
"#;

    #[test]
    fn one_child_serves_multiple_requests_and_index_then_closes() {
        let (root, path) = mock(ECHO);
        let mut session = Session::start(&path).unwrap();
        let pid = session.child.id();
        assert!(
            session
                .exchange(vec!["expired".into()], false, Instant::now(), MAX)
                .is_err()
        );
        assert_eq!(session.next, 0);
        for text in ["first", "second"] {
            let bytes = session
                .exchange(
                    vec![text.into()],
                    false,
                    Instant::now() + Duration::from_secs(3),
                    MAX,
                )
                .unwrap();
            assert_eq!(bytes, format!("{pid}:{text}").as_bytes());
        }
        let bytes = session
            .exchange(
                vec!["third".into(), "fourth".into()],
                true,
                Instant::now() + Duration::from_secs(3),
                MAX,
            )
            .unwrap();
        assert_eq!(bytes, format!("{pid}:thirdfourth").as_bytes());
        session.finish().unwrap();
        assert!(session.stopped && session.child.try_wait().unwrap().is_some());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cumulative_byte_bounds_apply_across_requests() {
        let (root, path) = mock(ECHO);
        let mut session = Session::start(&path).unwrap();
        let reply = session.exchange(vec!["first".into()], false,
            Instant::now() + Duration::from_secs(3), MAX).unwrap();
        assert_eq!(session.output_bytes, reply.len());
        assert!(session.input_bytes > "first".len());
        // Advance accounting to its boundary without allocating half a GiB.
        session.input_bytes = SESSION_BYTES;
        let sequence = session.next;
        assert!(session.exchange(vec!["too much input".into()], false,
            Instant::now() + Duration::from_secs(3), MAX).unwrap_err().contains("cumulative input"));
        assert_eq!(session.next, sequence); // Refused before dispatch.
        session.input_bytes = 0;
        session.output_bytes = SESSION_BYTES - 1;
        assert!(session.exchange(vec!["too much output".into()], false,
            Instant::now() + Duration::from_secs(3), MAX).unwrap_err().contains("byte bound"));
        assert!(session.stopped); // An over-budget peer cannot supply later results.
        assert_eq!(session.output_bytes, SESSION_BYTES - 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn deadline_stops_owned_child_and_does_not_replay() {
        let (root, path) = mock("import time\ntime.sleep(30)");
        let mut session = Session::start(&path).unwrap();
        let pid = session.child.id();
        let started = Instant::now();
        assert!(
            session
                .exchange(
                    vec!["wait".into()],
                    false,
                    started + Duration::from_millis(100),
                    MAX
                )
                .is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(session.stopped);
        assert!(
            session
                .exchange(
                    vec!["retry".into()],
                    false,
                    Instant::now() + Duration::from_secs(1),
                    MAX
                )
                .is_err()
        );
        assert_eq!(session.child.id(), pid);
        fs::remove_dir_all(root).unwrap();
    }

    fn slow(seconds: &str) -> String {
        ECHO.replace("import sys,os", "import sys,os,time")
            .replace(" output=str(", &format!(" time.sleep({seconds})\n output=str("))
    }

    #[test]
    fn stall_deadline_applies_per_request_and_names_the_request() {
        let (root, path) = mock(&slow("0.3"));
        let mut lane = Lane::with(path.clone(), "probe", SESSION_BYTES, Duration::from_secs(1));
        for i in 0..5 {
            let reply = lane.request(vec![format!("r{i}")], MAX).unwrap();
            assert!(reply.ends_with(format!(":r{i}").as_bytes()));
        }
        assert_eq!(lane.finish().unwrap(), 1);
        let mut shared = Session::start(&path).unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        assert!((0..5).any(|i| shared.exchange(vec![format!("r{i}")], true, deadline, MAX).is_err()));
        drop(shared);
        let (stuck_root, stuck) = mock(&slow("2"));
        let mut lane = Lane::with(stuck, "probe", SESSION_BYTES, Duration::from_secs(1));
        let started = Instant::now();
        let error = lane.request(vec!["late".into()], MAX).unwrap_err();
        assert!(started.elapsed() < Duration::from_millis(1900));
        assert!(error.starts_with("probe request 0: "), "{error}");
        assert_eq!(crate::error_code(&error), "deadline_exceeded");
        drop(lane);
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(stuck_root).unwrap();
    }

    #[test]
    fn early_core_exit_reports_status_and_stderr_as_core_unavailable() {
        let (root, path) = mock("import sys\nsys.stderr.write('allocation failed: heap exhausted\\n')\nsys.exit(3)");
        let mut session = Session::start(&path).unwrap();
        let error = session.exchange(vec!["x".into()], true, Instant::now() + Duration::from_secs(5), MAX).unwrap_err();
        assert!(error.contains("; core diagnosis: exit status: 3; stderr: allocation failed: heap exhausted"), "{error}");
        assert_eq!(crate::error_code(&error), "core_unavailable");
        // A process group whose only member is a zombie refuses kill with EPERM on macOS.
        let mut child = Command::new("/usr/bin/true").process_group(0).spawn().unwrap();
        std::thread::sleep(Duration::from_secs(1));
        let result = unsafe { libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL) };
        assert_eq!((result, std::io::Error::last_os_error().raw_os_error()), (-1, Some(libc::EPERM)));
        child.wait().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lane_rotates_cores_before_its_byte_budget() {
        let (root, path) = mock(ECHO);
        let mut lane = Lane::with(path, "probe", 200, Duration::from_secs(3));
        let mut pids = std::collections::BTreeSet::new();
        for i in 0..6 {
            let payload = format!("request-{i}-").repeat(4);
            let reply = String::from_utf8(lane.request(vec![payload.clone()], 64).unwrap()).unwrap();
            let (pid, echoed) = reply.split_once(':').unwrap();
            assert_eq!(echoed, payload);
            pids.insert(pid.to_string());
        }
        assert!(pids.len() >= 2, "{pids:?}");
        assert_eq!(lane.finish().unwrap(), pids.len());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn request_count_rotates_independent_lane_and_refuses_retained_lane() {
        let (root, path) = mock(ECHO);
        let mut independent = Lane::with(path.clone(), "probe", SESSION_BYTES, Duration::from_secs(3));
        let first = String::from_utf8(independent.request(vec!["first".into()], 64).unwrap()).unwrap();
        independent.session.as_mut().unwrap().next = CORE_REQUESTS;
        let second = String::from_utf8(independent.request(vec!["second".into()], 64).unwrap()).unwrap();
        assert_ne!(first.split_once(':').unwrap().0, second.split_once(':').unwrap().0);
        assert!(second.ends_with(":second"));
        assert_eq!(independent.finish().unwrap(), 2);

        let mut retained = Lane::with(path, "retained", SESSION_BYTES, Duration::from_secs(3));
        retained.request_one("first".into(), 64).unwrap();
        retained.session.as_mut().unwrap().next = CORE_REQUESTS;
        let error = retained.request_one("second".into(), 64).unwrap_err();
        assert!(error.contains("retry from revalidated input"), "{error}");
        assert_eq!(retained.cores, 1);
        // The synthetic count cannot be closed normally; this is a refusal control.
        retained.session.take().unwrap().stop().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unexpected_final_response_cannot_be_hidden_by_successful_exit() {
        let body = ECHO.replace(
            "sys.stdout.buffer.flush()",
            "sys.stdout.buffer.write(b\"S1\\t\"+seq+b\"\\tdone\\t1\\nx\")\n sys.stdout.buffer.flush()",
        );
        let (root, path) = mock(&body);
        let mut session = Session::start(&path).unwrap();
        session
            .exchange(
                vec!["value".into()],
                false,
                Instant::now() + Duration::from_secs(3),
                MAX,
            )
            .unwrap();
        assert!(session.finish().is_err());
        assert!(session.stopped);
        fs::remove_dir_all(root).unwrap();
    }
}
