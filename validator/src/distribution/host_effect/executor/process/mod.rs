use super::super::PinnedHostExecutable;
use super::super::lifecycle::{
    DescriptorExecutionCapability, DescriptorExecutionPlatform, DescriptorExecutionPrimitive,
};
use super::model::{
    CommandCapture, HostEffectCancellation, HostEffectExecutionPolicy, HostEffectExecutorErrorId,
};
use crate::distribution::HostCommand;

include!("backend_failure.rs");

include!("execute_retained_descriptor.rs");

include!("create_pipe.rs");

pub(crate) fn execute_bounded_observation(
    backend: &mut dyn RetainedDescriptorProcessBackend,
    capability: &DescriptorExecutionCapability,
    executable: &PinnedHostExecutable,
    command: &HostCommand,
    policy: &HostEffectExecutionPolicy,
    cancellation: &HostEffectCancellation,
    cwd: std::os::fd::RawFd,
) -> Result<CommandCapture, BackendFailure> {
    backend.execute(capability, executable, command, policy, cancellation, cwd)
}
