use crate::distribution::HostCommand;
use crate::distribution::host_effect::executor::{
    CommandCapture, HostEffectCancellation, HostEffectExecutionPolicy, HostEffectExecutorErrorId,
};
use std::io;
use std::os::fd::RawFd;
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
    let mut child = ChildGuard::new(spawned.pid);
    let mut pipes = spawned.pipes;
    if pipes.prepare_parent().is_err() {
        return Err(child.failure(&mut pipes, HostEffectExecutorErrorId::ProcessSpawnFailed));
    }
    // The sandboxed child is held suspended until parent-side capture resources
    // are closed and ready for bounded reads.
    if unsafe { libc::kill(-child.pid, libc::SIGCONT) } != 0 {
        return Err(child.failure(&mut pipes, HostEffectExecutorErrorId::ProcessSpawnFailed));
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut status = 0;
    let deadline = Instant::now() + policy.timeout();
    let mut stdout_closed = false;
    let mut stderr_closed = false;
    loop {
        if cancellation.is_cancelled() {
            return Err(child.failure(&mut pipes, HostEffectExecutorErrorId::Cancelled));
        }
        if Instant::now() >= deadline {
            return Err(child.failure(&mut pipes, timeout_error_id()));
        }
        match pipes.drain_stdout(&mut stdout, policy.stdout_limit()) {
            Ok(closed) => stdout_closed |= closed,
            Err(id) => return Err(child.failure(&mut pipes, id)),
        }
        match pipes.drain_stderr(&mut stderr, policy.stderr_limit()) {
            Ok(closed) => stderr_closed |= closed,
            Err(id) => return Err(child.failure(&mut pipes, id)),
        }
        let waited = unsafe { libc::waitpid(child.pid, &mut status, libc::WNOHANG) };
        if waited == child.pid {
            child.reaped = true;
            break;
        }
        if waited < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
            return Err(child.failure(&mut pipes, HostEffectExecutorErrorId::ProcessFailed));
        }
        pipes.poll(stdout_closed, stderr_closed);
    }
    if !recovery::settle(child.pid, child.reaped) {
        return Err(child.failure(&mut pipes, HostEffectExecutorErrorId::ProcessFailed));
    }
    let drain_deadline = Instant::now() + Duration::from_secs(1);
    while !(stdout_closed && stderr_closed) && Instant::now() < drain_deadline {
        match pipes.drain_stdout(&mut stdout, policy.stdout_limit()) {
            Ok(closed) => stdout_closed |= closed,
            Err(id) => return Err(child.failure(&mut pipes, id)),
        }
        match pipes.drain_stderr(&mut stderr, policy.stderr_limit()) {
            Ok(closed) => stderr_closed |= closed,
            Err(id) => return Err(child.failure(&mut pipes, id)),
        }
        if !(stdout_closed && stderr_closed) {
            pipes.poll(stdout_closed, stderr_closed);
        }
    }
    if !(stdout_closed && stderr_closed) || recovery::group_exists(child.pid) {
        return Err(child.failure(&mut pipes, HostEffectExecutorErrorId::ProcessFailed));
    }
    pipes.close_capture();
    child.settled = true;
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

struct ChildGuard {
    pid: libc::pid_t,
    reaped: bool,
    settled: bool,
}

impl ChildGuard {
    fn new(pid: libc::pid_t) -> Self {
        Self {
            pid,
            reaped: false,
            settled: false,
        }
    }

    fn failure(
        &mut self,
        pipes: &mut pipes::Pipes,
        id: HostEffectExecutorErrorId,
    ) -> DarwinFailure {
        let cleanup_ok = recovery::settle(self.pid, self.reaped);
        self.settled = cleanup_ok;
        pipes.close_all();
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

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if !self.settled {
            let _ = recovery::settle(self.pid, self.reaped);
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
