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
    use crate::process_custody::{
        DarwinExecutionFailure, DarwinExecutionPolicy, execute_suspended_descriptor,
    };
    let (path, device, inode) = executable.raw_loaded_identity();
    let environment = command
        .environment()
        .iter()
        .cloned()
        .collect::<std::collections::BTreeMap<_, _>>();
    let result = execute_suspended_descriptor(
        path,
        device,
        inode,
        cwd,
        command.argv(),
        &environment,
        DarwinExecutionPolicy {
            timeout: policy.timeout(),
            output_limit: policy.stdout_limit().max(policy.stderr_limit()),
        },
        || cancellation.is_cancelled(),
    );
    match result {
        Ok(capture) => Ok(CommandCapture {
            exit_code: capture.exit_code,
            stdout: capture.stdout,
            stderr: capture.stderr,
        }),
        Err(failure) => {
            let id = match failure {
                DarwinExecutionFailure::Spawn => HostEffectExecutorErrorId::ProcessSpawnFailed,
                DarwinExecutionFailure::LoadedVnode => {
                    HostEffectExecutorErrorId::ExecutableMutation
                }
                DarwinExecutionFailure::Resume => HostEffectExecutorErrorId::ProcessSpawnFailed,
                DarwinExecutionFailure::Cancelled => HostEffectExecutorErrorId::Cancelled,
                DarwinExecutionFailure::Timeout => HostEffectExecutorErrorId::Timeout,
                DarwinExecutionFailure::OutputOverflow => HostEffectExecutorErrorId::OutputOverflow,
                DarwinExecutionFailure::Process => HostEffectExecutorErrorId::ProcessFailed,
                DarwinExecutionFailure::Cleanup => HostEffectExecutorErrorId::ProcessFailed,
            };
            Err(BackendFailure {
                id,
                started: !matches!(failure, DarwinExecutionFailure::Spawn),
                capture: CommandCapture::empty_failure(),
            })
        }
    }
}
