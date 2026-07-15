use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

const CONTENDER_POLL_CADENCE: Duration = Duration::from_millis(5);
const CONTENDER_CLEANUP_BOUND: Duration = Duration::from_secs(1);
const MAX_CAPTURE_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub(crate) enum BoundedContender {
    Exited(Output),
    TerminatedAndReaped(Output),
    ExitedAtDeadline(Output),
    CleanupFailed(&'static str),
}

#[derive(Clone, Copy)]
enum Completion {
    Exited,
    Terminated,
    ExitedAtDeadline,
}

pub(crate) fn run_bounded_contender(
    command: &mut Command,
    execution_bound: Duration,
) -> BoundedContender {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let pipes = match Pipes::from_child(&mut child) {
        Ok(pipes) => pipes,
        Err(cause) => return terminate_after_pipe_failure(&mut child, cause),
    };
    wait_for_contender(child, pipes, execution_bound)
}

fn wait_for_contender(
    mut child: Child,
    mut pipes: Pipes,
    execution_bound: Duration,
) -> BoundedContender {
    let deadline = Instant::now() + execution_bound;
    loop {
        if let Err(cause) = pipes.drain_available() {
            return terminate_after_pipe_failure(&mut child, cause);
        }
        if Instant::now() >= deadline {
            return terminate_and_reap(&mut child, pipes);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                return pipes.finish(
                    status,
                    Instant::now() + CONTENDER_CLEANUP_BOUND,
                    Completion::Exited,
                );
            }
            Ok(None) => {}
            Err(_) => {
                return terminate_after_pipe_failure(&mut child, "contender-reap-status-failed");
            }
        }
        std::thread::sleep(CONTENDER_POLL_CADENCE);
    }
}

fn terminate_and_reap(child: &mut Child, mut pipes: Pipes) -> BoundedContender {
    if child.kill().is_err() {
        return match child.try_wait() {
            Ok(Some(status)) => pipes.finish(
                status,
                Instant::now() + CONTENDER_CLEANUP_BOUND,
                Completion::ExitedAtDeadline,
            ),
            Ok(None) => BoundedContender::CleanupFailed("contender-kill-failed-while-live"),
            Err(_) => BoundedContender::CleanupFailed("contender-reap-status-failed"),
        };
    }
    let deadline = Instant::now() + CONTENDER_CLEANUP_BOUND;
    loop {
        if let Err(cause) = pipes.drain_available() {
            return BoundedContender::CleanupFailed(cause);
        }
        match child.try_wait() {
            Ok(Some(status)) => return pipes.finish(status, deadline, Completion::Terminated),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(CONTENDER_POLL_CADENCE),
            Ok(None) => return BoundedContender::CleanupFailed("contender-reap-deadline-exceeded"),
            Err(_) => return BoundedContender::CleanupFailed("contender-reap-status-failed"),
        }
    }
}

fn terminate_after_pipe_failure(child: &mut Child, cause: &'static str) -> BoundedContender {
    if child.kill().is_ok() {
        let deadline = Instant::now() + CONTENDER_CLEANUP_BOUND;
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => return BoundedContender::CleanupFailed(cause),
                Ok(None) => {}
                Err(_) => return BoundedContender::CleanupFailed("contender-reap-status-failed"),
            }
            std::thread::sleep(CONTENDER_POLL_CADENCE);
        }
    }
    BoundedContender::CleanupFailed(cause)
}

struct Pipes {
    stdout: ChildStdout,
    stderr: ChildStderr,
    stdout_closed: bool,
    stderr_closed: bool,
    stdout_bytes: Vec<u8>,
    stderr_bytes: Vec<u8>,
}

impl Pipes {
    fn from_child(child: &mut Child) -> Result<Self, &'static str> {
        let stdout = child.stdout.take().ok_or("contender-stdout-missing")?;
        let stderr = child.stderr.take().ok_or("contender-stderr-missing")?;
        set_nonblocking(&stdout)?;
        set_nonblocking(&stderr)?;
        Ok(Self {
            stdout,
            stderr,
            stdout_closed: false,
            stderr_closed: false,
            stdout_bytes: Vec::new(),
            stderr_bytes: Vec::new(),
        })
    }

    fn drain_available(&mut self) -> Result<(), &'static str> {
        self.stdout_closed = self.stdout_closed || drain(&mut self.stdout, &mut self.stdout_bytes)?;
        self.stderr_closed = self.stderr_closed || drain(&mut self.stderr, &mut self.stderr_bytes)?;
        Ok(())
    }

    fn finish(
        mut self,
        status: ExitStatus,
        deadline: Instant,
        completion: Completion,
    ) -> BoundedContender {
        loop {
            if self.drain_available().is_err() {
                return BoundedContender::CleanupFailed("contender-pipe-drain-failed");
            }
            if self.stdout_closed && self.stderr_closed {
                let output = Output {
                    status,
                    stdout: self.stdout_bytes,
                    stderr: self.stderr_bytes,
                };
                return match completion {
                    Completion::Exited => BoundedContender::Exited(output),
                    Completion::Terminated => BoundedContender::TerminatedAndReaped(output),
                    Completion::ExitedAtDeadline => BoundedContender::ExitedAtDeadline(output),
                };
            }
            if Instant::now() >= deadline {
                return BoundedContender::CleanupFailed("contender-pipe-drain-deadline-exceeded");
            }
            std::thread::sleep(CONTENDER_POLL_CADENCE);
        }
    }
}

fn set_nonblocking(file: &impl AsRawFd) -> Result<(), &'static str> {
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        Err("contender-pipe-nonblocking-failed")
    } else {
        Ok(())
    }
}

fn drain(reader: &mut impl Read, captured: &mut Vec<u8>) -> Result<bool, &'static str> {
    let mut buffer = [0_u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(true),
            Ok(read) => {
                let remaining = MAX_CAPTURE_BYTES.saturating_sub(captured.len());
                captured.extend_from_slice(&buffer[..read.min(remaining)]);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(false),
            Err(_) => return Err("contender-pipe-read-failed"),
        }
    }
}

#[cfg(test)]
#[path = "contender_process_controls.rs"]
mod controls;
