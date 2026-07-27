use crate::distribution::host_effect::executor::HostEffectExecutorErrorId;
use std::io;
use std::os::fd::RawFd;

pub(super) struct Pipes {
    stdin_read: RawFd,
    stdin_write: RawFd,
    stdout_read: RawFd,
    stdout_write: RawFd,
    stderr_read: RawFd,
    stderr_write: RawFd,
}

impl Pipes {
    pub(super) fn new() -> io::Result<Self> {
        let stdin = pipe()?;
        let stdout = match pipe() {
            Ok(value) => value,
            Err(error) => {
                close_pair(stdin);
                return Err(error);
            }
        };
        let stderr = match pipe() {
            Ok(value) => value,
            Err(error) => {
                close_pair(stdin);
                close_pair(stdout);
                return Err(error);
            }
        };
        Ok(Self {
            stdin_read: stdin.0,
            stdin_write: stdin.1,
            stdout_read: stdout.0,
            stdout_write: stdout.1,
            stderr_read: stderr.0,
            stderr_write: stderr.1,
        })
    }

    pub(super) fn stdin_read(&self) -> RawFd {
        self.stdin_read
    }
    pub(super) fn stdout_write(&self) -> RawFd {
        self.stdout_write
    }
    pub(super) fn stderr_write(&self) -> RawFd {
        self.stderr_write
    }

    pub(super) fn prepare_parent(&mut self) -> io::Result<()> {
        self.close_stdin_read();
        self.close_stdout_write();
        self.close_stderr_write();
        self.close_stdin_write();
        set_nonblocking(self.stdout_read)?;
        set_nonblocking(self.stderr_read)
    }

    pub(super) fn drain_stdout(
        &self,
        output: &mut Vec<u8>,
        limit: usize,
    ) -> Result<bool, HostEffectExecutorErrorId> {
        drain(self.stdout_read, output, limit)
    }

    pub(super) fn drain_stderr(
        &self,
        output: &mut Vec<u8>,
        limit: usize,
    ) -> Result<bool, HostEffectExecutorErrorId> {
        drain(self.stderr_read, output, limit)
    }

    pub(super) fn poll(&self, stdout_closed: bool, stderr_closed: bool) {
        let mut descriptors = [
            libc::pollfd {
                fd: self.stdout_read,
                events: if stdout_closed {
                    0
                } else {
                    libc::POLLIN | libc::POLLHUP
                },
                revents: 0,
            },
            libc::pollfd {
                fd: self.stderr_read,
                events: if stderr_closed {
                    0
                } else {
                    libc::POLLIN | libc::POLLHUP
                },
                revents: 0,
            },
        ];
        // SAFETY: descriptors points to writable pollfd storage for this call.
        let _ = unsafe { libc::poll(descriptors.as_mut_ptr(), descriptors.len() as _, 10) };
    }

    pub(super) fn close_capture(&mut self) {
        close_fd(self.stdout_read);
        close_fd(self.stderr_read);
        self.stdout_read = -1;
        self.stderr_read = -1;
    }

    pub(super) fn close_all(&mut self) {
        close_fd(self.stdin_read);
        close_fd(self.stdin_write);
        close_fd(self.stdout_read);
        close_fd(self.stdout_write);
        close_fd(self.stderr_read);
        close_fd(self.stderr_write);
        self.stdin_read = -1;
        self.stdin_write = -1;
        self.stdout_read = -1;
        self.stdout_write = -1;
        self.stderr_read = -1;
        self.stderr_write = -1;
    }

    fn close_stdin_read(&mut self) {
        close_fd(self.stdin_read);
        self.stdin_read = -1;
    }
    fn close_stdin_write(&mut self) {
        close_fd(self.stdin_write);
        self.stdin_write = -1;
    }
    fn close_stdout_write(&mut self) {
        close_fd(self.stdout_write);
        self.stdout_write = -1;
    }
    fn close_stderr_write(&mut self) {
        close_fd(self.stderr_write);
        self.stderr_write = -1;
    }
}

impl Drop for Pipes {
    fn drop(&mut self) {
        self.close_all();
    }
}

fn pipe() -> io::Result<(RawFd, RawFd)> {
    let mut descriptors = [-1; 2];
    // SAFETY: descriptors is writable storage for two owned pipe descriptors.
    if unsafe { libc::pipe(descriptors.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    for descriptor in descriptors {
        // SAFETY: descriptor is owned by this function until returned or closed.
        if unsafe { libc::fcntl(descriptor, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            close_pair((descriptors[0], descriptors[1]));
            return Err(io::Error::last_os_error());
        }
    }
    Ok((descriptors[0], descriptors[1]))
}

fn close_pair(pair: (RawFd, RawFd)) {
    close_fd(pair.0);
    close_fd(pair.1);
}

fn set_nonblocking(descriptor: RawFd) -> io::Result<()> {
    // SAFETY: descriptor remains owned by Pipes for these fcntl calls.
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn drain(
    descriptor: RawFd,
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<bool, HostEffectExecutorErrorId> {
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        // SAFETY: buffer is writable storage for the read and descriptor is a
        // live nonblocking capture pipe.
        let count = unsafe {
            libc::read(
                descriptor,
                buffer.as_mut_ptr().cast(),
                buffer.len() as libc::size_t,
            )
        };
        if count > 0 {
            let count = count as usize;
            if output.len().saturating_add(count) > limit {
                return Err(HostEffectExecutorErrorId::OutputOverflow);
            }
            output.extend_from_slice(&buffer[..count]);
            continue;
        }
        if count == 0 {
            return Ok(true);
        }
        match io::Error::last_os_error().raw_os_error() {
            Some(libc::EINTR) => continue,
            Some(libc::EAGAIN) => return Ok(false),
            _ => return Err(HostEffectExecutorErrorId::ProcessFailed),
        }
    }
}

fn close_fd(descriptor: RawFd) {
    if descriptor >= 0 {
        // SAFETY: each descriptor is closed at most once by Pipes ownership.
        unsafe {
            libc::close(descriptor);
        }
    }
}
