use crate::distribution::HostCommand;
use crate::distribution::host_effect::executor::{
    CommandCapture, HostEffectCancellation, HostEffectExecutionPolicy, HostEffectExecutorErrorId,
};
use std::io;
use std::os::fd::RawFd;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::time::{Duration, Instant};

#[path = "darwin_pipes.rs"]
mod pipes;
#[path = "darwin_recovery.rs"]
mod recovery;
#[path = "darwin_sandbox.rs"]
mod sandbox;
#[path = "darwin_spawn.rs"]
mod spawn;
#[cfg(test)]
#[path = "darwin_custody_tests.rs"]
mod tests;

pub(super) struct DarwinFailure {
    pub(super) id: HostEffectExecutorErrorId,
    pub(super) started: bool,
    pub(super) capture: CommandCapture,
}

pub(super) fn execute(
    path: &Path,
    command: &HostCommand,
    policy: &HostEffectExecutionPolicy,
    cancellation: &HostEffectCancellation,
    cwd: RawFd,
) -> Result<CommandCapture, DarwinFailure> {
    if cancellation.is_cancelled() {
        return Err(before_start(HostEffectExecutorErrorId::Cancelled));
    }
    let spawned = spawn::spawn(path, command, cwd)
        .map_err(|_| before_start(HostEffectExecutorErrorId::ProcessSpawnFailed))?;
    let mut session = ChildSession::new(spawned);
    match catch_unwind(AssertUnwindSafe(|| {
        session.run(command, policy, cancellation)
    })) {
        Ok(result) => result,
        Err(_) => Err(session.failure(HostEffectExecutorErrorId::ProcessFailed)),
    }
}

struct ChildSession {
    pid: libc::pid_t,
    reaped: bool,
    scope_terminal: bool,
    finalized: bool,
    pipes: Option<pipes::Pipes>,
}

impl ChildSession {
    fn new(spawned: spawn::Spawned) -> Self {
        Self {
            pid: spawned.pid,
            reaped: false,
            scope_terminal: false,
            finalized: false,
            pipes: Some(spawned.pipes),
        }
    }

    fn run(
        &mut self,
        _command: &HostCommand,
        policy: &HostEffectExecutionPolicy,
        cancellation: &HostEffectCancellation,
    ) -> Result<CommandCapture, DarwinFailure> {
        if self
            .pipes
            .as_mut()
            .expect("live child pipes")
            .prepare_parent()
            .is_err()
        {
            return Err(self.failure(HostEffectExecutorErrorId::ProcessSpawnFailed));
        }
        // The sandboxed child is held suspended until parent-side capture resources
        // are closed and ready for bounded reads.
        if unsafe { libc::kill(-self.pid, libc::SIGCONT) } != 0 {
            return Err(self.failure(HostEffectExecutorErrorId::ProcessSpawnFailed));
        }

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut status = 0;
        let deadline = Instant::now() + policy.timeout();
        let mut stdout_closed = false;
        let mut stderr_closed = false;
        loop {
            if cancellation.is_cancelled() {
                return Err(self.failure(HostEffectExecutorErrorId::Cancelled));
            }
            if Instant::now() >= deadline {
                return Err(self.failure(timeout_error_id()));
            }
            match self
                .pipes
                .as_ref()
                .expect("live child pipes")
                .drain_stdout(&mut stdout, policy.stdout_limit())
            {
                Ok(closed) => stdout_closed |= closed,
                Err(id) => return Err(self.failure(id)),
            }
            match self
                .pipes
                .as_ref()
                .expect("live child pipes")
                .drain_stderr(&mut stderr, policy.stderr_limit())
            {
                Ok(closed) => stderr_closed |= closed,
                Err(id) => return Err(self.failure(id)),
            }
            let waited = unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG) };
            if waited == self.pid {
                self.reaped = true;
                break;
            }
            if waited < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return Err(self.failure(HostEffectExecutorErrorId::ProcessFailed));
            }
            self.pipes
                .as_ref()
                .expect("live child pipes")
                .poll(stdout_closed, stderr_closed);
        }

        if !recovery::settle(self.pid, self.reaped) {
            return Err(self.failure(HostEffectExecutorErrorId::ProcessFailed));
        }
        self.scope_terminal = true;
        let drain_deadline = Instant::now() + Duration::from_secs(1);
        while !(stdout_closed && stderr_closed) && Instant::now() < drain_deadline {
            match self
                .pipes
                .as_ref()
                .expect("live child pipes")
                .drain_stdout(&mut stdout, policy.stdout_limit())
            {
                Ok(closed) => stdout_closed |= closed,
                Err(id) => return Err(self.failure(id)),
            }
            match self
                .pipes
                .as_ref()
                .expect("live child pipes")
                .drain_stderr(&mut stderr, policy.stderr_limit())
            {
                Ok(closed) => stderr_closed |= closed,
                Err(id) => return Err(self.failure(id)),
            }
            if !(stdout_closed && stderr_closed) {
                self.pipes
                    .as_ref()
                    .expect("live child pipes")
                    .poll(stdout_closed, stderr_closed);
            }
        }
        if !(stdout_closed && stderr_closed) || recovery::group_exists(self.pid) {
            return Err(self.failure(HostEffectExecutorErrorId::ProcessFailed));
        }
        self.pipes
            .as_mut()
            .expect("live child pipes")
            .close_capture();
        self.pipes.take();
        self.finalized = true;
        let capture = CommandCapture {
            exit_code: exit_code(status),
            stdout,
            stderr,
        };
        if capture.exit_code() != 0 {
            return Err(DarwinFailure {
                id: HostEffectExecutorErrorId::ProcessFailed,
                started: true,
                capture,
            });
        }
        Ok(capture)
    }

    fn failure(&mut self, id: HostEffectExecutorErrorId) -> DarwinFailure {
        let cleanup_ok = self.scope_terminal || recovery::settle(self.pid, self.reaped);
        self.scope_terminal |= cleanup_ok;
        if let Some(pipes) = self.pipes.as_mut() {
            pipes.close_all();
        }
        self.pipes.take();
        self.finalized = true;
        DarwinFailure {
            id: if cleanup_ok {
                id
            } else {
                HostEffectExecutorErrorId::ProcessFailed
            },
            started: true,
            capture: CommandCapture::empty_failure(),
        }
    }
}

fn exit_code(status: i32) -> i32 {
    if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else if libc::WIFSIGNALED(status) {
        128 + libc::WTERMSIG(status)
    } else {
        -1
    }
}

fn before_start(id: HostEffectExecutorErrorId) -> DarwinFailure {
    DarwinFailure {
        id,
        started: false,
        capture: CommandCapture::empty_failure(),
    }
}

fn timeout_error_id() -> HostEffectExecutorErrorId {
    #[cfg(test)]
    {
        HostEffectExecutorErrorId::Timeout
    }
    #[cfg(not(test))]
    {
        HostEffectExecutorErrorId::ProcessFailed
    }
}
