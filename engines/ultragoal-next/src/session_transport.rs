//! Opt-in foreground computation lifetime. No verifier/provider or authority effects.
use super::*;
use std::io::ErrorKind;
use std::os::unix::{
    ffi::OsStrExt,
    fs::{DirBuilderExt, FileTypeExt, PermissionsExt},
    io::AsRawFd,
    net::{UnixListener, UnixStream},
};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
const HEADER_MAX: usize = 8192;
/// Owner lifetime and idle bounds in seconds: (default, least, most).
const LIFETIME_SECONDS: (u64, u64, u64) = (300, 30, 7200);
const IDLE_SECONDS: (u64, u64, u64) = (30, 5, 1800);
const REQUESTS: usize = 4096;
const TRANSFER_MAX: usize = 256 * 1024 * 1024;
#[derive(Clone, Debug, Eq, PartialEq)]
struct Stamp {
    dev: u64,
    ino: u64,
    size: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}
fn stamp(path: &Path) -> Result<Stamp, String> {
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    Ok(Stamp {
        dev: m.dev(),
        ino: m.ino(),
        size: m.len(),
        mtime: (m.mtime(), m.mtime_nsec()),
        ctime: (m.ctime(), m.ctime_nsec()),
    })
}
fn object_id(path: &Path) -> Result<String, String> {
    let s = stamp(path)?;
    Ok(format!("{}:{}", s.dev, s.ino))
}
// Cleanup only objects created by this owner whose identities still match.
// This also covers startup failures before the normal close path is reached.
struct Lease {
    directory: PathBuf,
    identity: String,
    remove_directory: bool,
    files: Vec<(PathBuf, String)>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        for (path, id) in self.files.iter().rev() {
            if object_id(path).ok().as_ref() == Some(id) {
                let _ = fs::remove_file(path);
            }
        }
        if self.remove_directory && object_id(&self.directory).ok().as_ref() == Some(&self.identity) {
            let _ = fs::remove_dir(&self.directory);
        }
    }
}
fn root_id(id: fs_adapter::Identity) -> String {
    format!("{}:{}", id.device, id.inode)
}
fn peer(stream: &UnixStream) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let (mut uid, mut gid) = (0, 0);
        if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } != 0
            || uid != unsafe { libc::geteuid() }
        {
            return Err("session peer UID unavailable or different".into());
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = stream;
        Err("local session peer checks are qualified only on macOS".into())
    }
}
fn endpoint(path: &Path) -> Result<String, String> {
    let m = fs::symlink_metadata(path).map_err(|e| format!("explicit session unavailable: {e}"))?;
    let parent = path.parent().ok_or("session directory")?;
    let d = fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
    let uid = unsafe { libc::geteuid() };
    if !m.file_type().is_socket()
        || m.mode() & 0o077 != 0
        || m.uid() != uid
        || !d.is_dir()
        || d.mode() & 0o077 != 0
        || d.uid() != uid
    {
        return Err(
            "session endpoint must be in an owner-only directory with a restricted socket".into(),
        );
    }
    Ok(format!("{}:{}:{}:{}", d.dev(), d.ino(), m.dev(), m.ino()))
}
/// Waits until `fd` is readable (or hung up) for at most `millis`; loop
/// conditions are rechecked by the caller after every return.
fn wait_readable(fd: std::os::fd::RawFd, millis: i32) -> bool {
    let mut entry = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
    unsafe { libc::poll(&mut entry, 1, millis) > 0 }
}
fn polling(stream: &UnixStream) -> Result<(), String> {
    stream
        .set_nonblocking(false)
        .and_then(|_| stream.set_read_timeout(Some(Duration::from_millis(10))))
        .and_then(|_| stream.set_write_timeout(Some(Duration::from_millis(10))))
        .map_err(|e| e.to_string())
}
fn active(deadline: Instant) -> Result<(), String> {
    if is_interrupted() || Instant::now() >= deadline {
        Err("session cancelled or deadline exceeded".into())
    } else {
        Ok(())
    }
}
fn read_exact(
    stream: &mut UnixStream,
    mut bytes: &mut [u8],
    deadline: Instant,
) -> Result<(), String> {
    while !bytes.is_empty() {
        active(deadline)?;
        match stream.read(bytes) {
            Ok(0) => return Err("session disconnected during response/request".into()),
            Ok(n) => bytes = &mut bytes[n..],
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
fn write_all(stream: &mut UnixStream, mut bytes: &[u8], deadline: Instant) -> Result<(), String> {
    while !bytes.is_empty() {
        active(deadline)?;
        match stream.write(bytes) {
            Ok(0) => return Err("session write closed".into()),
            Ok(n) => bytes = &bytes[n..],
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
fn length(stream: &mut UnixStream, limit: usize, deadline: Instant) -> Result<usize, String> {
    let mut b = [0; 8];
    read_exact(stream, &mut b, deadline)?;
    let n = u64::from_be_bytes(b);
    if n > limit as u64 {
        return Err("session frame bound before allocation".into());
    }
    Ok(n as usize)
}
fn bytes(stream: &mut UnixStream, limit: usize, deadline: Instant) -> Result<Vec<u8>, String> {
    let n = length(stream, limit, deadline)?;
    let mut b = vec![0; n];
    read_exact(stream, &mut b, deadline)?;
    Ok(b)
}
fn write_bytes(stream: &mut UnixStream, b: &[u8], deadline: Instant) -> Result<(), String> {
    write_all(stream, &(b.len() as u64).to_be_bytes(), deadline)?;
    write_all(stream, b, deadline)
}
fn header(stream: &mut UnixStream, deadline: Instant) -> Result<Value, String> {
    provider::decode(&bytes(stream, HEADER_MAX, deadline)?)
}
fn write_header(stream: &mut UnixStream, v: &Value, deadline: Instant) -> Result<(), String> {
    let b = serde_json::to_vec(v).map_err(|e| e.to_string())?;
    if b.len() > HEADER_MAX {
        return Err("session header size".into());
    }
    write_bytes(stream, &b, deadline)
}
struct Remote {
    handle: Value,
    path: PathBuf,
    endpoint_id: String,
    root: fs_adapter::Root,
    root_identity: fs_adapter::Identity,
    core_path: PathBuf,
    core_stamp: Stamp,
    instance: String,
    next: u64,
    dead: bool,
    input_bytes: usize,
    output_bytes: usize,
    last_core_pid: Option<u64>,
}
static REMOTE: OnceLock<Mutex<Option<Remote>>> = OnceLock::new();
pub fn configure(args: &[String]) -> Result<(), String> {
    let Some(handle_path) = option(args, "--session")? else {
        return Ok(());
    };
    let verb = args.first().map(String::as_str).unwrap_or("");
    if !matches!(verb, "check" | "verify" | "index" | "select")
        || args.iter().any(|a| {
            matches!(
                a.as_str(),
                "--rust" | "--python" | "--typescript" | "--stdin-c" | "--bend-proof"
            )
        })
    {
        return Err("--session supports workspace check/verify/index/select commands".into());
    }
    let root_path = fs::canonicalize(option(args, "--root")?.ok_or("--session requires --root")?)
        .map_err(|e| e.to_string())?;
    let root = fs_adapter::Root::open(&root_path).map_err(|e| e.to_string())?;
    let root_identity = root.identity().map_err(|e| e.to_string())?;
    let handle = provider::decode(&read(Path::new(&handle_path), HEADER_MAX).map_err(|e| {
        format!(
            "explicit session handle unavailable; start a new session or deliberately run cold: {e}"
        )
    })?)?;
    if handle["schema"] != "ultragoal-session/1"
        || handle["workspace"].as_str() != root_path.to_str()
        || handle["workspace_identity"] != root_id(root_identity)
    {
        return Err("session workspace binding mismatch".into());
    }
    let path = PathBuf::from(handle["endpoint"].as_str().ok_or("session endpoint")?);
    let endpoint_id = endpoint(&path)?;
    if handle["endpoint_identity"] != endpoint_id {
        return Err("session endpoint was replaced".into());
    }
    let core_path = core()?;
    let core_stamp = stamp(&core_path)?;
    if handle["core_sha256"] != hash(&read(&core_path, MAX)?) || stamp(&core_path)? != core_stamp {
        return Err("session core differs; start a new session".into());
    }
    let remote = Remote {
        handle,
        path,
        endpoint_id,
        root,
        root_identity,
        core_path,
        core_stamp,
        instance: op_id()?,
        next: 0,
        dead: false,
        input_bytes: 0,
        output_bytes: 0,
        last_core_pid: None,
    };
    let mut cell = REMOTE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "session client lock poisoned")?;
    if cell.is_some() {
        return Err("session client already configured".into());
    }
    *cell = Some(remote);
    Ok(())
}
pub fn enabled() -> bool {
    REMOTE.get().is_some()
}
pub fn request(
    frames: Vec<String>,
    index: bool,
    deadline: Instant,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let cell = REMOTE.get().ok_or("session not configured")?;
    let mut guard = loop {
        active(deadline)?;
        match cell.try_lock() {
            Ok(guard) => break guard,
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err("session client lock poisoned".into());
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                std::thread::sleep(Duration::from_millis(2))
            }
        }
    };
    let r = guard.as_mut().ok_or("session client closed")?;
    let result = (|| {
        active(deadline)?;
        if r.dead
            || frames.is_empty()
            || frames.len() > 128
            || (!index && frames.len() != 1)
            || limit > REPORT_MAX
        {
            return Err("session request shape or closed client".into());
        }
        if !r.root.binding_current(r.root_identity)
            || endpoint(&r.path)? != r.endpoint_id
            || stamp(&r.core_path)? != r.core_stamp
        {
            return Err("session workspace/endpoint/core changed".into());
        }
        let total = frames.iter().try_fold(0usize, |n, f| {
            if f.is_empty() || f.len() > MAX {
                Err("session input frame bound")
            } else {
                n.checked_add(f.len())
                    .filter(|x| *x <= TRANSFER_MAX)
                    .ok_or("session input aggregate bound")
            }
        })?;
        let mut stream = UnixStream::connect(&r.path).map_err(|e| {
            format!(
                "explicit session unavailable; run cold deliberately or start a new session: {e}"
            )
        })?;
        peer(&stream)?;
        polling(&stream)?;
        let remaining = deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(30000) as u64;
        if remaining == 0 {
            return Err("session request deadline expired".into());
        }
        let seq = r.next;
        r.next = r.next.checked_add(1).ok_or("session sequence overflow")?;
        write_header(
            &mut stream,
            &json!({"schema":"ultragoal-session-request/1","incarnation":r.handle["incarnation"],"workspace_identity":r.handle["workspace_identity"],"core_sha256":r.handle["core_sha256"],"instance":r.instance,"sequence":seq,"index":index,"frames":frames.len(),"output_limit":limit,"remaining_ms":remaining}),
            deadline,
        )?;
        for frame in frames {
            write_bytes(&mut stream, frame.as_bytes(), deadline)?
        }
        let response = header(&mut stream, deadline)?;
        if response["schema"] != "ultragoal-session-response/1"
            || response["incarnation"] != r.handle["incarnation"]
            || response["instance"] != r.instance
            || response["sequence"] != seq
        {
            return Err("stale or substituted session response".into());
        }
        let output = bytes(&mut stream, limit, deadline)?;
        active(deadline)?;
        if !r.root.binding_current(r.root_identity)
            || endpoint(&r.path)? != r.endpoint_id
            || stamp(&r.core_path)? != r.core_stamp
        {
            return Err("session binding changed before admission".into());
        }
        r.input_bytes = r
            .input_bytes
            .checked_add(total)
            .ok_or("session accounting overflow")?;
        r.output_bytes = r
            .output_bytes
            .checked_add(output.len())
            .ok_or("session accounting overflow")?;
        r.last_core_pid = response["core_pid"].as_u64();
        Ok(output)
    })();
    if result.is_err() {
        r.dead = true;
    }
    result
}
pub fn report() -> Option<Value> {
    let guard = REMOTE.get()?.lock().ok()?;
    let r = guard.as_ref()?;
    Some(
        json!({"mode":"explicit-local-retained-computation","incarnation":r.handle["incarnation"],"owner_pid":r.handle["owner_pid"],"core_pid":r.last_core_pid,"frontend_pid":std::process::id(),"requests":r.next,"input_bytes":r.input_bytes,"output_bytes":r.output_bytes,"failed":r.dead,"trust":"reported local session under trusted-running-code/input assumptions; no protected host/adoption provenance"}),
    )
}
pub fn finish() -> Result<(), String> {
    Ok(())
}
fn instance(value: &Value) -> Result<&str, String> {
    let s = value.as_str().ok_or("session instance")?;
    if s.len() != 32
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("session instance shape".into());
    }
    Ok(s)
}
fn serve_request(
    mut stream: UnixStream,
    meta: &Value,
    root: &fs_adapter::Root,
    root_identity: fs_adapter::Identity,
    core_path: &Path,
    core_stamp: &Stamp,
    path: &Path,
    endpoint_id: &str,
    absolute: Instant,
    seen: &mut BTreeMap<String, u64>,
) -> Result<(), String> {
    peer(&stream)?;
    polling(&stream)?;
    let started = Instant::now();
    let h = header(
        &mut stream,
        (started + Duration::from_secs(3)).min(absolute),
    )?;
    let allowed = [
        "schema",
        "incarnation",
        "workspace_identity",
        "core_sha256",
        "instance",
        "sequence",
        "index",
        "frames",
        "output_limit",
        "remaining_ms",
    ];
    let object = h.as_object().ok_or("session header object")?;
    if object.len() != allowed.len()
        || object.keys().any(|k| !allowed.contains(&k.as_str()))
        || h["schema"] != "ultragoal-session-request/1"
    {
        return Err("session header schema".into());
    }
    for key in ["incarnation", "workspace_identity", "core_sha256"] {
        if h[key] != meta[key] {
            return Err(format!("session binding mismatch: {key}"));
        }
    }
    let id = instance(&h["instance"])?;
    let sequence = h["sequence"].as_u64().ok_or("session sequence")?;
    let expected = seen.get(id).copied().unwrap_or(0);
    if sequence != expected {
        return Err("duplicate or out-of-order session request".into());
    }
    seen.insert(
        id.to_string(),
        sequence.checked_add(1).ok_or("sequence overflow")?,
    );
    let remaining = h["remaining_ms"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 30000)
        .ok_or("session deadline bound")?;
    let deadline = (started + Duration::from_millis(remaining)).min(absolute);
    let count = h["frames"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 128)
        .ok_or("session frame count")?;
    let index = h["index"].as_bool().ok_or("session index flag")?;
    if !index && count != 1 {
        return Err("single core request requires one frame".into());
    }
    let limit = h["output_limit"]
        .as_u64()
        .filter(|n| *n <= REPORT_MAX as u64)
        .ok_or("session output limit")? as usize;
    let mut frames = Vec::new();
    let mut total = 0usize;
    for _ in 0..count {
        let n = length(&mut stream, MAX, deadline)?;
        if n == 0 {
            return Err("empty core frame".into());
        }
        total = total
            .checked_add(n)
            .filter(|n| *n <= TRANSFER_MAX)
            .ok_or("session input aggregate bound")?;
        let mut b = vec![0; n];
        read_exact(&mut stream, &mut b, deadline)?;
        frames.push(String::from_utf8(b).map_err(|_| "session frame UTF-8")?)
    }
    if !root.binding_current(root_identity)
        || stamp(core_path)? != *core_stamp
        || endpoint(path)? != endpoint_id
    {
        return Err("session binding changed before dispatch".into());
    }
    let watched = stream.try_clone().map_err(|e| e.to_string())?;
    let done = Arc::new(AtomicBool::new(false));
    let stopped = done.clone();
    let watcher = std::thread::spawn(move || {
        while !stopped.load(Ordering::SeqCst) {
            let mut b = 0u8;
            let n = unsafe {
                libc::recv(
                    watched.as_raw_fd(),
                    (&mut b as *mut u8).cast(),
                    1,
                    libc::MSG_PEEK | libc::MSG_DONTWAIT,
                )
            };
            if n >= 0 {
                INTERRUPTED.store(true, Ordering::SeqCst);
                break;
            }
            wait_readable(watched.as_raw_fd(), 1);
        }
    });
    let answer = core_session::request_local(frames, index, deadline, limit);
    done.store(true, Ordering::SeqCst);
    watcher
        .join()
        .map_err(|_| "session disconnect watcher panicked")?;
    let answer = answer?;
    active(deadline)?;
    if !root.binding_current(root_identity)
        || stamp(core_path)? != *core_stamp
        || endpoint(path)? != endpoint_id
    {
        return Err("session binding changed before publication".into());
    }
    write_header(
        &mut stream,
        &json!({"schema":"ultragoal-session-response/1","incarnation":meta["incarnation"],"instance":id,"sequence":sequence,"core_pid":core_session::observed_pid()}),
        deadline,
    )?;
    write_bytes(&mut stream, &answer, deadline)
}
fn seconds(args: &[String], flag: &str, (default, least, most): (u64, u64, u64)) -> Result<u64, String> {
    option(args, flag)?.map_or(Ok(default), |v| {
        v.parse::<u64>().ok().filter(|s| (least..=most).contains(s)).ok_or_else(|| {
            format!("session lifetime refused: --lifetime-seconds {}-{}, --idle-seconds {}-{}", LIFETIME_SECONDS.1, LIFETIME_SECONDS.2, IDLE_SECONDS.1, IDLE_SECONDS.2)
        })
    })
}
/// sun_path holds 104 bytes including the terminating NUL.
const SOCKET_PATH_MAX: usize = 103;
/// The socket lives in the session directory unless that path is too long for
/// sun_path; then it lives in the per-user temporary directory under a name derived
/// from the session directory. The handle names the endpoint either way, so clients
/// keep reading it from there and keep the same endpoint and peer checks.
fn socket_path(dir: &Path) -> Result<PathBuf, String> {
    let near = dir.join("core.sock");
    if near.as_os_str().len() <= SOCKET_PATH_MAX {
        return Ok(near);
    }
    let temp = env::temp_dir();
    let short = temp.join(format!("ug-{}.sock", &hash(dir.as_os_str().as_bytes())[..16]));
    let private = fs::symlink_metadata(&temp)
        .is_ok_and(|m| m.is_dir() && m.mode() & 0o077 == 0 && m.uid() == unsafe { libc::geteuid() });
    if short.as_os_str().len() > SOCKET_PATH_MAX || !private {
        return Err(format!(
            "session socket path {} exceeds the {SOCKET_PATH_MAX}-byte sun_path limit, and the fallback {} is too long or not in an owner-only directory; choose a shorter --directory",
            near.display(),
            short.display()
        ));
    }
    Ok(short)
}
pub fn serve(args: &[String]) -> Result<(Value, i32), String> {
    if option(args, "--session")?.is_some() {
        return Err("a foreground owner cannot connect to another session".into());
    }
    let lifetime = seconds(args, "--lifetime-seconds", LIFETIME_SECONDS)?;
    let idle = seconds(args, "--idle-seconds", IDLE_SECONDS)?;
    let started = Instant::now();
    let root_path = fs::canonicalize(option(args, "--root")?.ok_or("session --root required")?)
        .map_err(|e| e.to_string())?;
    let root = fs_adapter::Root::open(&root_path).map_err(|e| e.to_string())?;
    let root_identity = root.identity().map_err(|e| e.to_string())?;
    let requested = PathBuf::from(
        option(args, "--directory")?.ok_or("session --directory NEW_DIRECTORY required")?,
    );
    let parent = fs::canonicalize(requested.parent().ok_or("session directory parent")?)
        .map_err(|e| e.to_string())?;
    let dir = parent.join(requested.file_name().ok_or("session directory name")?);
    if dir.starts_with(&root_path) {
        return Err("session directory must be outside the observed workspace".into());
    }
    let created = match fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => true,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            let meta = fs::symlink_metadata(&dir).map_err(|e| e.to_string())?;
            if !meta.is_dir() || meta.file_type().is_symlink()
                || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0
                || fs::read_dir(&dir).map_err(|e| e.to_string())?.next().is_some()
            {
                return Err("session directory must be empty and owner-only".into());
            }
            false
        }
        Err(error) => return Err(error.to_string()),
    };
    let mut lease = Lease {
        directory: dir.clone(),
        identity: object_id(&dir)?,
        remove_directory: created,
        files: Vec::new(),
    };
    let path = socket_path(&dir)?;
    let listener = UnixListener::bind(&path)
        .map_err(|e| format!("session socket refused: {e}; the host must allow local Unix sockets; paths {}", path.display()))?;
    lease.files.push((path.clone(), object_id(&path)?));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    let endpoint_id = endpoint(&path)?;
    if unsafe { libc::listen(listener.as_raw_fd(), 8) } != 0 {
        return Err("session backlog bound unavailable".into());
    }
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let core_path = core()?;
    let core_stamp = stamp(&core_path)?;
    let core_digest = hash(&read(&core_path, MAX)?);
    if stamp(&core_path)? != core_stamp {
        return Err("core changed during owner startup".into());
    }
    core_session::enable_rotation();
    core_session::request_local(
        vec!["EXCLUSIONS\n".into()],
        false,
        Instant::now() + Duration::from_secs(3),
        MAX,
    )?;
    let meta = json!({"schema":"ultragoal-session/1","endpoint":path,"endpoint_identity":endpoint_id,"incarnation":op_id()?,"workspace":root_path,"workspace_identity":root_id(root_identity),"core_sha256":core_digest,"owner_pid":std::process::id(),"core_pid":core_session::observed_pid(),"startup_ms":started.elapsed().as_secs_f64()*1000.,"bounds":{"absolute_seconds":lifetime,"idle_seconds":idle,"requests":REQUESTS,"single_request_ms":30000,"queue_backlog":8,"core_cumulative_bytes_per_direction":512*1024*1024},"trust":"local reported computation session; no protected host/adoption provenance"});
    let handle_path = dir.join("session.json");
    let mut handle = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&handle_path)
        .map_err(|e| e.to_string())?;
    lease
        .files
        .push((handle_path.clone(), object_id(&handle_path)?));
    handle
        .write_all(&serde_json::to_vec(&meta).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    handle.sync_all().map_err(|e| e.to_string())?;
    drop(handle);
    let handle_id = object_id(&handle_path)?;
    println!(
        "{}",
        json!({"event":"session-ready","handle":handle_path,"session":meta})
    );
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let absolute = started + Duration::from_secs(lifetime);
    let mut count = 0usize;
    let mut seen = BTreeMap::new();
    let mut last = Instant::now();
    let result = (|| {
        while !is_interrupted()
            && Instant::now() < absolute
            && last.elapsed() < Duration::from_secs(idle)
            && count < REQUESTS
        {
            if !root.binding_current(root_identity)
                || stamp(&core_path)? != core_stamp
                || endpoint(&path)? != endpoint_id
            {
                return Err("foreground session binding changed".into());
            }
            if !wait_readable(listener.as_raw_fd(), 25) {
                core_session::maintain()?;
                continue;
            }
            let (stream, _) = match listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e.to_string()),
            };
            serve_request(
                stream,
                &meta,
                &root,
                root_identity,
                &core_path,
                &core_stamp,
                &path,
                &endpoint_id,
                absolute,
                &mut seen,
            )?;
            count += 1;
            last = Instant::now();
            core_session::maintain()?;
        }
        Ok(())
    })();
    let core_close = core_session::finish_local();
    drop(listener);
    let mut cleanup = Vec::new();
    if endpoint(&path).ok().as_deref() == Some(&endpoint_id) {
        if let Err(e) = fs::remove_file(&path) {
            cleanup.push(e.to_string())
        }
    } else {
        cleanup.push("changed endpoint preserved".into())
    }
    if object_id(&handle_path).ok().as_deref() == Some(&handle_id)
        && let Err(e) = fs::remove_file(&handle_path)
    {
        cleanup.push(e.to_string())
    }
    if created && let Err(e) = fs::remove_dir(&dir) {
        cleanup.push(format!("session directory retained: {e}"))
    }
    let error = result.err().or(core_close.err());
    Ok((
        json!({"schema":"ultragoal-session/1","state":if error.is_some(){"unavailable"}else{"closed"},"requests":count,"incarnation":meta["incarnation"],"error":error,"cleanup":cleanup,"retained_state":"discarded with owned core"}),
        if error.is_some() { 2 } else { 0 },
    ))
}
