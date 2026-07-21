use super::super::SelectedCodexExecutable;
use super::super::lifecycle::DescriptorExecutionCapability;
use super::model::{
    CommandCapture, HostEffectCancellation, HostEffectExecutionPolicy, HostEffectExecutorErrorId,
};
use crate::distribution::HostCommand;

include!("backend_failure.rs");

pub(crate) fn execute_bounded_observation(
    backend: &mut dyn RetainedDescriptorProcessBackend,
    capability: &DescriptorExecutionCapability,
    executable: &SelectedCodexExecutable,
    command: &HostCommand,
    policy: &HostEffectExecutionPolicy,
    cancellation: &HostEffectCancellation,
    cwd: std::os::fd::RawFd,
) -> Result<CommandCapture, BackendFailure> {
    backend.execute(capability, executable, command, policy, cancellation, cwd)
}
