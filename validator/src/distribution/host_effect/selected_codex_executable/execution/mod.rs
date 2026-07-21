use super::super::SelectedCodexExecutable;
use crate::distribution::HostCommand;
use crate::distribution::host_effect::executor::{
    BackendFailure, CommandCapture, HostEffectCancellation, HostEffectExecutionPolicy,
    HostEffectExecutorErrorId,
};
use crate::distribution::host_effect::lifecycle::{
    DescriptorExecutionCapability, DescriptorExecutionPlatform, DescriptorExecutionPrimitive,
};

#[cfg(target_os = "macos")]
mod darwin;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
mod descriptor;
mod process_group;

pub(super) fn execute(
    executable: &SelectedCodexExecutable,
    capability: &DescriptorExecutionCapability,
    command: &HostCommand,
    policy: &HostEffectExecutionPolicy,
    cancellation: &HostEffectCancellation,
    cwd: std::os::fd::RawFd,
) -> Result<CommandCapture, BackendFailure> {
    if cancellation.is_cancelled() {
        return Err(BackendFailure::before_start(
            HostEffectExecutorErrorId::Cancelled,
        ));
    }
    if command.program() != "codex" || command.argv().is_empty() {
        return Err(BackendFailure::before_start(
            HostEffectExecutorErrorId::ProcessSpawnFailed,
        ));
    }
    #[cfg(target_os = "linux")]
    {
        if capability.platform() != DescriptorExecutionPlatform::Linux
            || capability.primitive() != DescriptorExecutionPrimitive::ExecveAtEmptyPath
        {
            return Err(BackendFailure::before_start(
                HostEffectExecutorErrorId::UnsupportedPlatform,
            ));
        }
        return descriptor::execute(
            executable,
            command,
            policy,
            cancellation,
            command.environment(),
            DescriptorExecutionPrimitive::ExecveAtEmptyPath,
        );
    }
    #[cfg(target_os = "freebsd")]
    {
        if capability.platform() != DescriptorExecutionPlatform::FreeBsd
            || capability.primitive() != DescriptorExecutionPrimitive::Fexecve
        {
            return Err(BackendFailure::before_start(
                HostEffectExecutorErrorId::UnsupportedPlatform,
            ));
        }
        return descriptor::execute(
            executable,
            command,
            policy,
            cancellation,
            command.environment(),
            DescriptorExecutionPrimitive::Fexecve,
        );
    }
    #[cfg(target_os = "macos")]
    {
        if capability.platform() != DescriptorExecutionPlatform::Darwin
            || capability.primitive()
                != DescriptorExecutionPrimitive::DarwinPosixSpawnSuspendedLoadedVnode
        {
            return Err(BackendFailure::before_start(
                HostEffectExecutorErrorId::UnsupportedPlatform,
            ));
        }
        return darwin::execute(executable, command, policy, cancellation, cwd);
    }
    #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
    {
        let _ = (capability, executable, policy, cwd);
        Err(BackendFailure::before_start(
            HostEffectExecutorErrorId::UnsupportedPlatform,
        ))
    }
}
