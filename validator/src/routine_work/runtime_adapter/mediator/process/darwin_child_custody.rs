use std::io;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

pub(crate) struct BoundChild {
    pub(super) pid: libc::pid_t,
    pub(super) status: Option<ExitStatus>,
}

impl BoundChild {
    pub(crate) fn id(&self) -> u32 {
        self.pid as u32
    }

    pub(crate) fn pid(&self) -> libc::pid_t {
        self.pid
    }

    pub(crate) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if let Some(status) = self.status {
            return Ok(Some(status));
        }
        self.observe_wait(libc::WNOHANG)
    }

    pub(crate) fn wait(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.status {
            return Ok(status);
        }
        loop {
            if let Some(status) = self.observe_wait(0)? {
                return Ok(status);
            }
        }
    }

    pub(crate) fn kill(&mut self) -> io::Result<()> {
        // SAFETY: self.pid is the child PID returned by posix_spawn; kill only performs a signal.
        if unsafe { libc::kill(self.pid, libc::SIGKILL) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn observe_wait(&mut self, options: i32) -> io::Result<Option<ExitStatus>> {
        let mut raw = 0;
        // SAFETY: self.pid is a child PID, raw is writable, and options are waitpid flags.
        let result = unsafe { libc::waitpid(self.pid, &mut raw, options) };
        if result == 0 {
            return Ok(None);
        }
        if result == self.pid {
            let status = ExitStatus::from_raw(raw);
            self.status = Some(status);
            return Ok(Some(status));
        }
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::Interrupted {
            return self.observe_wait(options);
        }
        Err(error)
    }
}
