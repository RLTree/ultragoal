pub(super) struct BackendFailure {
    pub(super) id: HostEffectExecutorErrorId,
    pub(super) started: bool,
    pub(super) capture: CommandCapture,
}

impl BackendFailure {
    pub(super) fn before_start(id: HostEffectExecutorErrorId) -> Self {
        Self {
            id,
            started: false,
            capture: CommandCapture {
                exit_code: -1,
                stdout: Vec::new(),
                stderr: Vec::new(),
            },
        }
    }
}

/// The backend receives the retained executable descriptor and exact accepted
/// argv. It has no environment parameter: every successful implementation
/// executes with the policy-bound empty environment.
pub(super) trait RetainedDescriptorProcessBackend {
    fn execute(
        &mut self,
        capability: &DescriptorExecutionCapability,
        executable: &PinnedHostExecutable,
        command: &HostCommand,
        policy: &HostEffectExecutionPolicy,
        cancellation: &HostEffectCancellation,
    ) -> Result<CommandCapture, BackendFailure>;
}

#[derive(Default)]
pub(super) struct NativeRetainedDescriptorProcessBackend;

impl RetainedDescriptorProcessBackend for NativeRetainedDescriptorProcessBackend {
    fn execute(
        &mut self,
        capability: &DescriptorExecutionCapability,
        executable: &PinnedHostExecutable,
        command: &HostCommand,
        policy: &HostEffectExecutionPolicy,
        cancellation: &HostEffectCancellation,
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
            return execute_retained_descriptor(
                executable,
                command,
                policy,
                cancellation,
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
            return execute_retained_descriptor(
                executable,
                command,
                policy,
                cancellation,
                DescriptorExecutionPrimitive::Fexecve,
            );
        }

        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        {
            let _ = (capability, executable, policy);
            // The accepted lifecycle coordinator refuses Darwin before it
            // contacts this adapter. This second boundary is intentional
            // defense in depth and performs no fork, spawn, or write.
            Err(BackendFailure::before_start(
                HostEffectExecutorErrorId::UnsupportedPlatform,
            ))
        }
    }
}
