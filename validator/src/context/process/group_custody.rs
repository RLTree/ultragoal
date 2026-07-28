use super::bounded_capture::Captured;
use std::io;
use std::os::fd::AsRawFd;
use std::process::{Child, ChildStderr, ChildStdout};
use std::time::{Duration, Instant};

const CLEANUP_BUDGET: Duration = Duration::from_millis(500);

pub(super) fn drain_after_exit(
    stdout: &mut ChildStdout,
    stderr: &mut ChildStderr,
    captured_stdout: &mut Captured,
    captured_stderr: &mut Captured,
) {
    let deadline = Instant::now() + CLEANUP_BUDGET;
    while (!captured_stdout.eof || !captured_stderr.eof) && Instant::now() < deadline {
        let _ = captured_stdout.drain(stdout);
        let _ = captured_stderr.drain(stderr);
        if !captured_stdout.eof || !captured_stderr.eof {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

pub(super) fn terminate(
    child: &mut Child,
    group: i32,
    stdout: &mut ChildStdout,
    stderr: &mut ChildStderr,
) -> io::Result<()> {
    signal_group(group, libc::SIGKILL)?;
    let deadline = Instant::now() + CLEANUP_BUDGET;
    loop {
        let reaped = child.try_wait()?.is_some();
        let mut discard_stdout = Captured::new();
        let mut discard_stderr = Captured::new();
        let _ = discard_stdout.drain(stdout);
        let _ = discard_stderr.drain(stderr);
        if reaped && !group_exists(group) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::other("process-group cleanup timed out"));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub(super) fn terminate_without_drain(child: &mut Child, group: i32) -> io::Result<()> {
    signal_group(group, libc::SIGKILL)?;
    let deadline = Instant::now() + CLEANUP_BUDGET;
    loop {
        let reaped = child.try_wait()?.is_some();
        if reaped && !group_exists(group) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(io::Error::other("process-group cleanup timed out"));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

pub(super) fn set_nonblocking(stream: &impl AsRawFd) -> io::Result<()> {
    // SAFETY: the descriptor belongs to the live pipe stream and fcntl has no
    // pointer arguments for these commands.
    let flags = unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor remains live and flags is the value returned by
    // F_GETFL with the nonblocking bit added.
    if unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(super) fn signal_group(group: i32, signal: i32) -> io::Result<()> {
    // SAFETY: group is the positive identity captured directly from the child.
    if unsafe { libc::kill(-group, signal) } == 0 {
        Ok(())
    } else if io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub(super) fn group_exists(group: i32) -> bool {
    // SAFETY: signal zero probes only the captured process group.
    (unsafe { libc::kill(-group, 0) }) == 0
        || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}
