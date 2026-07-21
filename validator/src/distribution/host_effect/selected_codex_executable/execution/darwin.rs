use super::super::SelectedCodexExecutable;
use crate::distribution::HostCommand;
use crate::distribution::host_effect::executor::{
    BackendFailure, CommandCapture, HostEffectCancellation, HostEffectExecutionPolicy,
    HostEffectExecutorErrorId,
};

#[cfg(target_os = "macos")]
pub(super) fn execute(
    executable: &SelectedCodexExecutable,
    command: &HostCommand,
    policy: &HostEffectExecutionPolicy,
    cancellation: &HostEffectCancellation,
    cwd: std::os::fd::RawFd,
) -> Result<CommandCapture, BackendFailure> {
    let _ = (
        executable,
        command,
        policy.timeout(),
        policy.stdout_limit(),
        policy.stderr_limit(),
        cancellation.is_cancelled(),
        cwd,
    );
    // Darwin has no byte-sealing executable handoff in this candidate. A
    // loaded-vnode/path check is only identity evidence and cannot prevent a
    // same-inode rewrite after final validation, so refuse before spawn.
    Err(BackendFailure::before_start(
        HostEffectExecutorErrorId::UnsupportedPlatform,
    ))
}
