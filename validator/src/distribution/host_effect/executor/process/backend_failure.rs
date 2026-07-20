pub(crate) struct BackendFailure {
    pub(super) id: HostEffectExecutorErrorId,
    pub(super) started: bool,
    pub(super) capture: CommandCapture,
}

impl BackendFailure {
    pub(super) fn before_start(id: HostEffectExecutorErrorId) -> Self {
        Self {
            id,
            started: false,
            capture: CommandCapture::empty_failure(),
        }
    }
}

/// The backend receives the retained executable descriptor and exact accepted
/// argv. The cwd descriptor is the exact retained host target root; environment
/// entries are policy-bound by the accepted command plan.
pub(crate) trait RetainedDescriptorProcessBackend {
    fn execute(
        &mut self,
        capability: &DescriptorExecutionCapability,
        executable: &PinnedHostExecutable,
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
        executable: &PinnedHostExecutable,
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
            let _ = cwd;
            return execute_retained_descriptor(
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
            let _ = cwd;
            return execute_retained_descriptor(
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
            return execute_darwin(executable, command, policy, cancellation, cwd);
        }
        #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
        {
            let _ = (capability, executable, policy, cwd);
            Err(BackendFailure::before_start(
                HostEffectExecutorErrorId::UnsupportedPlatform,
            ))
        }
    }
}

#[cfg(target_os = "macos")]
fn execute_darwin(
    executable: &PinnedHostExecutable,
    command: &HostCommand,
    policy: &HostEffectExecutionPolicy,
    cancellation: &HostEffectCancellation,
    cwd: std::os::fd::RawFd,
) -> Result<CommandCapture, BackendFailure> {
    use crate::process_custody::{
        DarwinExecutionFailure, DarwinExecutionPolicy, execute_suspended_descriptor,
    };
    let (path, device, inode) = executable.loaded_identity();
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
