pub(crate) struct BackendFailure {
    pub(in crate::distribution::host_effect) id: HostEffectExecutorErrorId,
    pub(in crate::distribution::host_effect) started: bool,
    pub(in crate::distribution::host_effect) capture: CommandCapture,
}

impl BackendFailure {
    pub(in crate::distribution::host_effect) fn before_start(
        id: HostEffectExecutorErrorId,
    ) -> Self {
        Self {
            id,
            started: false,
            capture: CommandCapture::empty_failure(),
        }
    }
}

/// The backend receives typed command, policy, cancellation, capability, and
/// target inputs. Platform descriptor custody and process launch remain inside
/// the selected executable capability.
pub(crate) trait RetainedDescriptorProcessBackend {
    fn execute(
        &mut self,
        capability: &DescriptorExecutionCapability,
        executable: &SelectedCodexExecutable,
        command: &HostCommand,
        policy: &HostEffectExecutionPolicy,
        cancellation: &HostEffectCancellation,
        cwd: std::os::fd::RawFd,
    ) -> Result<CommandCapture, BackendFailure>;
}

#[derive(Default)]
pub(crate) struct NativeRetainedDescriptorProcessBackend;

impl RetainedDescriptorProcessBackend for NativeRetainedDescriptorProcessBackend {
    fn execute(
        &mut self,
        capability: &DescriptorExecutionCapability,
        executable: &SelectedCodexExecutable,
        command: &HostCommand,
        policy: &HostEffectExecutionPolicy,
        cancellation: &HostEffectCancellation,
        cwd: std::os::fd::RawFd,
    ) -> Result<CommandCapture, BackendFailure> {
        executable.execute(capability, command, policy, cancellation, cwd)
    }
}
