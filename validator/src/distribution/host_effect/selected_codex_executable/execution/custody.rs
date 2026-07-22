use crate::distribution::HostCommand;
use crate::distribution::host_effect::executor::{
    CommandCapture, HostEffectCancellation, HostEffectExecutionPolicy, HostEffectExecutorErrorId,
};
use std::os::fd::RawFd;
use std::path::Path;

#[path = "darwin_pipes.rs"]
mod pipes;
#[path = "darwin_recovery.rs"]
mod recovery;
#[path = "darwin_sandbox.rs"]
mod sandbox;
#[path = "darwin_session.rs"]
mod session;
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
    session::ChildSession::new(spawned).run_guarded(policy, cancellation)
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
