use super::super::SelectedCodexExecutable;
use crate::distribution::HostCommand;
use crate::distribution::host_effect::executor::{
    BackendFailure, CommandCapture, HostEffectCancellation, HostEffectExecutionPolicy,
    HostEffectExecutorErrorId,
};

#[path = "custody.rs"]
mod custody;

#[cfg(target_os = "macos")]
pub(super) fn execute(
    executable: &SelectedCodexExecutable,
    command: &HostCommand,
    policy: &HostEffectExecutionPolicy,
    cancellation: &HostEffectCancellation,
    cwd: std::os::fd::RawFd,
) -> Result<CommandCapture, BackendFailure> {
    executable
        .revalidate_launch()
        .map_err(|_| BackendFailure::before_start(HostEffectExecutorErrorId::ExecutableMutation))?;
    let capture =
        match custody::execute(executable.launch_path(), command, policy, cancellation, cwd) {
            Ok(capture) => capture,
            Err(failure) => {
                if executable.revalidate_launch().is_err() {
                    return Err(BackendFailure {
                        id: HostEffectExecutorErrorId::ExecutableMutation,
                        started: failure.started,
                        capture: failure.capture,
                    });
                }
                return Err(BackendFailure {
                    id: failure.id,
                    started: failure.started,
                    capture: failure.capture,
                });
            }
        };
    if executable.revalidate_launch().is_err() {
        return Err(BackendFailure {
            id: HostEffectExecutorErrorId::ExecutableMutation,
            started: true,
            capture,
        });
    }
    Ok(capture)
}
