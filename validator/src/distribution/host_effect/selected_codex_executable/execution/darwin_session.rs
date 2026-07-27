use super::{DarwinFailure, pipes, recovery, timeout_error_id};
use crate::distribution::host_effect::executor::{
    CommandCapture, HostEffectCancellation, HostEffectExecutionPolicy, HostEffectExecutorErrorId,
};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

pub(super) struct ChildSession {
    pid: libc::pid_t,
    reaped: bool,
    scope_terminal: bool,
    pipes: Option<pipes::Pipes>,
}

impl ChildSession {
    pub(super) fn new(spawned: super::spawn::Spawned) -> Self {
        Self {
            pid: spawned.pid,
            reaped: false,
            scope_terminal: false,
            pipes: Some(spawned.pipes),
        }
    }

    pub(super) fn run_guarded(
        self,
        policy: &HostEffectExecutionPolicy,
        cancellation: &HostEffectCancellation,
    ) -> Result<CommandCapture, DarwinFailure> {
        let mut session = self;
        match catch_unwind(AssertUnwindSafe(|| session.run(policy, cancellation))) {
            Ok(Ok(capture)) => session.finalize_success(capture),
            Ok(Err(id)) => Err(session.finalize_failure(id)),
            Err(_) => Err(session.finalize_failure(HostEffectExecutorErrorId::ProcessFailed)),
        }
    }

    fn run(
        &mut self,
        policy: &HostEffectExecutionPolicy,
        cancellation: &HostEffectCancellation,
    ) -> Result<CommandCapture, HostEffectExecutorErrorId> {
        if self
            .pipes
            .as_mut()
            .expect("live child pipes")
            .prepare_parent()
            .is_err()
        {
            return Err(HostEffectExecutorErrorId::ProcessSpawnFailed);
        }
        // The sandboxed child is held suspended until parent-side capture resources
        // are closed and ready for bounded reads.
        if unsafe { libc::kill(-self.pid, libc::SIGCONT) } != 0 {
            return Err(HostEffectExecutorErrorId::ProcessSpawnFailed);
        }

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut status = 0;
        let deadline = Instant::now() + policy.timeout();
        let mut stdout_closed = false;
        let mut stderr_closed = false;
        loop {
            if cancellation.is_cancelled() {
                return Err(HostEffectExecutorErrorId::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(timeout_error_id());
            }
            match self
                .pipes
                .as_ref()
                .expect("live child pipes")
                .drain_stdout(&mut stdout, policy.stdout_limit())
            {
                Ok(closed) => stdout_closed |= closed,
                Err(id) => return Err(id),
            }
            match self
                .pipes
                .as_ref()
                .expect("live child pipes")
                .drain_stderr(&mut stderr, policy.stderr_limit())
            {
                Ok(closed) => stderr_closed |= closed,
                Err(id) => return Err(id),
            }
            let waited = unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG) };
            if waited == self.pid {
                self.reaped = true;
                break;
            }
            if waited < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return Err(HostEffectExecutorErrorId::ProcessFailed);
            }
            self.pipes
                .as_ref()
                .expect("live child pipes")
                .poll(stdout_closed, stderr_closed);
        }

        if !recovery::settle(self.pid, self.reaped) {
            return Err(HostEffectExecutorErrorId::ProcessFailed);
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
                Err(id) => return Err(id),
            }
            match self
                .pipes
                .as_ref()
                .expect("live child pipes")
                .drain_stderr(&mut stderr, policy.stderr_limit())
            {
                Ok(closed) => stderr_closed |= closed,
                Err(id) => return Err(id),
            }
            if !(stdout_closed && stderr_closed) {
                self.pipes
                    .as_ref()
                    .expect("live child pipes")
                    .poll(stdout_closed, stderr_closed);
            }
        }
        if !(stdout_closed && stderr_closed) || recovery::group_exists(self.pid) {
            return Err(HostEffectExecutorErrorId::ProcessFailed);
        }
        Ok(CommandCapture {
            exit_code: exit_code(status),
            stdout,
            stderr,
        })
    }

    fn finalize_success(
        mut self,
        capture: CommandCapture,
    ) -> Result<CommandCapture, DarwinFailure> {
        self.pipes
            .as_mut()
            .expect("live child pipes")
            .close_capture();
        self.pipes.take();
        if capture.exit_code() != 0 {
            return Err(DarwinFailure {
                id: HostEffectExecutorErrorId::ProcessFailed,
                started: true,
                capture,
            });
        }
        Ok(capture)
    }

    pub(super) fn finalize_failure(mut self, id: HostEffectExecutorErrorId) -> DarwinFailure {
        let cleanup_ok = if self.scope_terminal && !recovery::group_exists(self.pid) {
            true
        } else {
            recovery::settle(self.pid, self.reaped)
        };
        if let Some(pipes) = self.pipes.as_mut() {
            pipes.close_all();
        }
        self.pipes.take();
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
