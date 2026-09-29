//! Physical I/O and lifecycle for fixed native adapters; no command selection.
use super::{MAX, is_interrupted};
use std::io::{Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(crate) struct Process {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub out: Vec<u8>,
    pub err: Vec<u8>,
    pub timeout: bool,
    pub cancelled: bool,
    pub truncated: bool,
    pub input_complete: bool,
    pub stdout_complete: bool,
    pub stderr_complete: bool,
    pub child_pid: u32,
    pub started_unix_ms: u128,
    pub ended_unix_ms: u128,
    pub ms: u128,
}

enum Stream {
    Input(bool),
    Output(Vec<u8>, bool),
    Error(Vec<u8>, bool),
}

fn timestamp() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis())
}

fn drain(mut input: impl Read, limit: usize) -> (Vec<u8>, bool) {
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    let mut complete = true;
    loop {
        match input.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                let keep = n.min(limit.saturating_sub(bytes.len()));
                bytes.extend_from_slice(&chunk[..keep]);
                complete &= keep == n;
            }
            Err(_) => { complete = false; break; }
        }
    }
    (bytes, complete)
}

pub(crate) fn run(command: Command, input: Vec<u8>, timeout: Duration) -> Result<Process, String> {
    run_bounded(command, input, timeout, MAX)
}

pub(crate) fn run_bounded(mut command: Command, input: Vec<u8>, timeout: Duration, output_limit: usize) -> Result<Process, String> {
    if is_interrupted() || timeout.is_zero() { return Err("native request cancelled or expired before dispatch".into()); }
    let start = Instant::now();
    let started_unix_ms = timestamp();
    command.process_group(0);
    let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| e.to_string())?;
    let child_pid = child.id();
    let mut stdin = child.stdin.take().ok_or("stdin")?;
    let stdout = child.stdout.take().ok_or("stdout")?;
    let stderr = child.stderr.take().ok_or("stderr")?;
    let (tx, rx) = mpsc::sync_channel(3);
    let input_tx = tx.clone();
    std::thread::spawn(move || {
        let complete = stdin.write_all(&input).and_then(|_| stdin.flush()).is_ok();
        drop(stdin);
        let _ = input_tx.send(Stream::Input(complete));
    });
    let output_tx = tx.clone();
    std::thread::spawn(move || {
        let (bytes, complete) = drain(stdout, output_limit);
        let _ = output_tx.send(Stream::Output(bytes, complete));
    });
    std::thread::spawn(move || {
        let (bytes, complete) = drain(stderr, MAX);
        let _ = tx.send(Stream::Error(bytes, complete));
    });
    let mut input_complete = None;
    let mut output = None;
    let mut error = None;
    let mut timed_out = false;
    let mut cancelled = false;
    let mut broken_stream = false;
    let mut receive = |event| match event {
        Stream::Input(complete) => input_complete = Some(complete),
        Stream::Output(bytes, complete) => output = Some((bytes, complete)),
        Stream::Error(bytes, complete) => error = Some((bytes, complete)),
    };
    // Do not reap the leader until all pipes close. Its reserved PID keeps any
    // inherited-stream descendants in an unambiguous owned process group.
    let mut events = 0;
    let status = loop {
        if is_interrupted() || start.elapsed() >= timeout {
            cancelled = is_interrupted(); timed_out = !cancelled; break None;
        }
        if events == 3 {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? { break Some(status); }
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }
        match rx.recv_timeout(Duration::from_millis(2)) {
            Ok(event) => { receive(event); events += 1; }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => { broken_stream = true; break None; }
        }
    };
    let status = match status {
        Some(status) => status,
        None => {
            let killed = unsafe { libc::kill(-(child_pid as libc::pid_t), libc::SIGKILL) };
            if killed != 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
                return Err(format!("owned process group {child_pid} termination unavailable; do not replay automatically"));
            }
            let cleanup = Instant::now() + Duration::from_millis(250);
            let mut status = None;
            while Instant::now() < cleanup {
                if status.is_none() { status = child.try_wait().map_err(|e| e.to_string())?; }
                if events < 3 {
                    match rx.recv_timeout(Duration::from_millis(2)) {
                        Ok(event) => { receive(event); events += 1; }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                } else if status.is_some() { break; }
                else { std::thread::sleep(Duration::from_millis(2)); }
            }
            status.ok_or_else(||format!("owned native child {child_pid} exit unconfirmed; do not replay automatically"))?
        }
    };
    drop(receive);
    // A pipe retained outside the owned group cannot make this result complete.
    // Reader handles are deliberately not joined without a completion event;
    // this bounded CLI returns unknown and exits instead of hanging indefinitely.
    let (out, stdout_complete) = output.unwrap_or_default();
    let (err, stderr_complete) = error.unwrap_or_default();
    let input_complete = input_complete.unwrap_or(false);
    Ok(Process {
        code: status.code(), signal: status.signal(), out, err,
        timeout: timed_out || start.elapsed() > timeout,
        cancelled: cancelled || is_interrupted(),
        truncated: broken_stream || !input_complete || !stdout_complete || !stderr_complete,
        input_complete, stdout_complete, stderr_complete, child_pid, started_unix_ms,
        ended_unix_ms: timestamp(), ms: start.elapsed().as_millis(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn python(script: &str) -> Command {
        let mut c = Command::new("/usr/bin/python3"); c.args(["-I", "-S", "-c", script]); c
    }
    #[test]
    fn exited_leader_with_inherited_pipes_stays_bounded_and_incomplete() {
        let start = Instant::now();
        let p = run(python("import os,time\nprint('success-looking output',flush=True)\nif os.fork()==0: time.sleep(30)\nelse: os._exit(0)"), Vec::new(), Duration::from_millis(100)).unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(p.timeout); assert_eq!(p.code, Some(0));
        assert!(p.out.starts_with(b"success-looking output"));
    }
    #[test]
    fn input_and_stream_completeness_are_observed_separately() {
        let p = run(python("import sys\nb=sys.stdin.buffer.read()\nsys.stdout.buffer.write(b)\nsys.stderr.write('diagnostic')"), b"captured\0bytes".to_vec(), Duration::from_secs(2)).unwrap();
        assert_eq!(p.out,b"captured\0bytes"); assert_eq!(p.err,b"diagnostic");
        assert!(p.input_complete && p.stdout_complete && p.stderr_complete && !p.truncated);
        assert!(p.child_pid>0 && p.ended_unix_ms>0);
        let p = run_bounded(python("print('long output')"), Vec::new(), Duration::from_secs(2), 2).unwrap();
        assert_eq!(p.out.len(),2); assert!(!p.stdout_complete && p.truncated);
    }
}
