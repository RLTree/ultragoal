use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::process::{Child, ChildStderr, ChildStdout, ExitStatus, Output};

pub(super) const MAX_CAPTURE_BYTES: usize = 64 * 1024;

pub(super) struct CapturedPipes {
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    stdout_closed: bool,
    stderr_closed: bool,
    stdout_bytes: Vec<u8>,
    stderr_bytes: Vec<u8>,
}

impl CapturedPipes {
    pub(super) fn take(child: &mut Child) -> (Self, Option<&'static str>) {
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let failure = match (&stdout, &stderr) {
            (None, _) => Some("contender-stdout-missing"),
            (_, None) => Some("contender-stderr-missing"),
            (Some(stdout), Some(stderr)) => set_nonblocking(stdout)
                .and_then(|()| set_nonblocking(stderr))
                .err(),
        };
        (
            Self {
                stdout_closed: stdout.is_none(),
                stderr_closed: stderr.is_none(),
                stdout,
                stderr,
                stdout_bytes: Vec::new(),
                stderr_bytes: Vec::new(),
            },
            failure,
        )
    }

    pub(super) fn drain_available(&mut self) -> Result<(), &'static str> {
        if let Some(stdout) = self.stdout.as_mut() {
            self.stdout_closed = self.stdout_closed || drain(stdout, &mut self.stdout_bytes)?;
        }
        if let Some(stderr) = self.stderr.as_mut() {
            self.stderr_closed = self.stderr_closed || drain(stderr, &mut self.stderr_bytes)?;
        }
        Ok(())
    }

    pub(super) fn closed(&self) -> bool {
        self.stdout_closed && self.stderr_closed
    }

    pub(super) fn into_output(self, status: ExitStatus) -> Output {
        Output {
            status,
            stdout: self.stdout_bytes,
            stderr: self.stderr_bytes,
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
    for _ in 0..16 {
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
    Ok(false)
}
