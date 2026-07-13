use super::super::PinnedHostExecutable;
use super::super::lifecycle::DescriptorExecutionCapability;
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use super::super::lifecycle::{DescriptorExecutionPlatform, DescriptorExecutionPrimitive};
use super::model::{
    CommandCapture, HostEffectCancellation, HostEffectExecutionPolicy, HostEffectExecutorErrorId,
};
use crate::distribution::HostCommand;

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
pub(crate) trait RetainedDescriptorProcessBackend {
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
pub(crate) struct NativeRetainedDescriptorProcessBackend;

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

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn execute_retained_descriptor(
    executable: &PinnedHostExecutable,
    command: &HostCommand,
    policy: &HostEffectExecutionPolicy,
    cancellation: &HostEffectCancellation,
    primitive: DescriptorExecutionPrimitive,
) -> Result<CommandCapture, BackendFailure> {
    use std::ffi::CString;
    use std::os::fd::AsRawFd;

    let mut arguments = Vec::with_capacity(command.argv().len() + 1);
    arguments.push(CString::new(command.program()).map_err(|_| {
        BackendFailure::before_start(HostEffectExecutorErrorId::ProcessSpawnFailed)
    })?);
    for argument in command.argv() {
        arguments.push(CString::new(argument.as_bytes()).map_err(|_| {
            BackendFailure::before_start(HostEffectExecutorErrorId::ProcessSpawnFailed)
        })?);
    }
    let mut argv = arguments
        .iter()
        .map(|argument| argument.as_ptr())
        .collect::<Vec<_>>();
    argv.push(std::ptr::null());
    let environment = [std::ptr::null::<libc::c_char>()];

    let (stdout_read, stdout_write) = create_pipe()?;
    let (stderr_read, stderr_write) = match create_pipe() {
        Ok(pipe) => pipe,
        Err(failure) => {
            close_fd(stdout_read);
            close_fd(stdout_write);
            return Err(failure);
        }
    };

    let child = unsafe { libc::fork() };
    if child < 0 {
        for descriptor in [stdout_read, stdout_write, stderr_read, stderr_write] {
            close_fd(descriptor);
        }
        return Err(BackendFailure::before_start(
            HostEffectExecutorErrorId::ProcessSpawnFailed,
        ));
    }
    if child == 0 {
        unsafe {
            libc::close(stdout_read);
            libc::close(stderr_read);
            if libc::setpgid(0, 0) != 0
                || libc::dup2(stdout_write, libc::STDOUT_FILENO) < 0
                || libc::dup2(stderr_write, libc::STDERR_FILENO) < 0
            {
                libc::_exit(126);
            }
            libc::close(stdout_write);
            libc::close(stderr_write);
            match primitive {
                #[cfg(target_os = "linux")]
                DescriptorExecutionPrimitive::ExecveAtEmptyPath => {
                    libc::execveat(
                        executable.file().as_raw_fd(),
                        c"".as_ptr(),
                        argv.as_ptr(),
                        environment.as_ptr(),
                        libc::AT_EMPTY_PATH,
                    );
                }
                #[cfg(target_os = "freebsd")]
                DescriptorExecutionPrimitive::Fexecve => {
                    libc::fexecve(
                        executable.file().as_raw_fd(),
                        argv.as_ptr(),
                        environment.as_ptr(),
                    );
                }
                _ => {}
            }
            libc::_exit(127);
        }
    }

    close_fd(stdout_write);
    close_fd(stderr_write);
    if set_nonblocking(stdout_read).is_err() || set_nonblocking(stderr_read).is_err() {
        terminate_process_group(child);
        reap(child);
        close_fd(stdout_read);
        close_fd(stderr_read);
        return Err(BackendFailure {
            id: HostEffectExecutorErrorId::ProcessSpawnFailed,
            started: true,
            capture: CommandCapture {
                exit_code: -1,
                stdout: Vec::new(),
                stderr: Vec::new(),
            },
        });
    }
    // Both the child and parent attempt group creation. EACCES/EPERM here can
    // mean the child won the race and already execed; every other failure is
    // handled conservatively by killing both the group and direct child.
    let _ = unsafe { libc::setpgid(child, child) };

    let started = std::time::Instant::now();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut status = 0;
    let failure = loop {
        if cancellation.is_cancelled() {
            break Some(HostEffectExecutorErrorId::Cancelled);
        }
        if started.elapsed() >= policy.timeout() {
            break Some(HostEffectExecutorErrorId::Timeout);
        }
        if let Err(id) = drain(stdout_read, &mut stdout, policy.stdout_limit()) {
            break Some(id);
        }
        if let Err(id) = drain(stderr_read, &mut stderr, policy.stderr_limit()) {
            break Some(id);
        }
        let waited = unsafe { libc::waitpid(child, &mut status, libc::WNOHANG) };
        if waited == child {
            break None;
        }
        if waited < 0 && last_errno() != Some(libc::EINTR) {
            break Some(HostEffectExecutorErrorId::ProcessFailed);
        }
        let mut descriptors = [
            libc::pollfd {
                fd: stdout_read,
                events: libc::POLLIN | libc::POLLHUP,
                revents: 0,
            },
            libc::pollfd {
                fd: stderr_read,
                events: libc::POLLIN | libc::POLLHUP,
                revents: 0,
            },
        ];
        let polled = unsafe { libc::poll(descriptors.as_mut_ptr(), descriptors.len() as _, 10) };
        if polled < 0 && last_errno() != Some(libc::EINTR) {
            break Some(HostEffectExecutorErrorId::ProcessFailed);
        }
    };

    if let Some(id) = failure {
        terminate_process_group(child);
        reap(child);
        let _ = drain(stdout_read, &mut stdout, policy.stdout_limit());
        let _ = drain(stderr_read, &mut stderr, policy.stderr_limit());
        close_fd(stdout_read);
        close_fd(stderr_read);
        return Err(BackendFailure {
            id,
            started: true,
            capture: CommandCapture {
                exit_code: -1,
                stdout,
                stderr,
            },
        });
    }

    // The direct child is already reaped, but a descendant may still retain
    // the process group or either capture pipe. End the group before accepting
    // the command as complete so no background descendant can escape the
    // transaction boundary.
    terminate_descendant_group(child);
    let _ = drain(stdout_read, &mut stdout, policy.stdout_limit());
    let _ = drain(stderr_read, &mut stderr, policy.stderr_limit());
    close_fd(stdout_read);
    close_fd(stderr_read);
    let exit_code = if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else if libc::WIFSIGNALED(status) {
        128 + libc::WTERMSIG(status)
    } else {
        -1
    };
    let capture = CommandCapture {
        exit_code,
        stdout,
        stderr,
    };
    if exit_code != 0 {
        return Err(BackendFailure {
            id: HostEffectExecutorErrorId::ProcessFailed,
            started: true,
            capture,
        });
    }
    Ok(capture)
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn create_pipe() -> Result<(libc::c_int, libc::c_int), BackendFailure> {
    let mut descriptors = [-1; 2];
    if unsafe { libc::pipe(descriptors.as_mut_ptr()) } != 0 {
        return Err(BackendFailure::before_start(
            HostEffectExecutorErrorId::ProcessSpawnFailed,
        ));
    }
    for descriptor in descriptors {
        if unsafe { libc::fcntl(descriptor, libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
            close_fd(descriptors[0]);
            close_fd(descriptors[1]);
            return Err(BackendFailure::before_start(
                HostEffectExecutorErrorId::ProcessSpawnFailed,
            ));
        }
    }
    Ok((descriptors[0], descriptors[1]))
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn set_nonblocking(descriptor: libc::c_int) -> Result<(), ()> {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } != 0
    {
        return Err(());
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn drain(
    descriptor: libc::c_int,
    output: &mut Vec<u8>,
    limit: usize,
) -> Result<(), HostEffectExecutorErrorId> {
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let count = unsafe {
            libc::read(
                descriptor,
                buffer.as_mut_ptr().cast(),
                buffer.len() as libc::size_t,
            )
        };
        if count > 0 {
            let count = count as usize;
            if output.len().saturating_add(count) > limit {
                return Err(HostEffectExecutorErrorId::OutputOverflow);
            }
            output.extend_from_slice(&buffer[..count]);
            continue;
        }
        if count == 0 {
            return Ok(());
        }
        match last_errno() {
            Some(libc::EINTR) => continue,
            Some(libc::EAGAIN) => return Ok(()),
            _ => return Err(HostEffectExecutorErrorId::ProcessFailed),
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn terminate_process_group(child: libc::pid_t) {
    unsafe {
        libc::kill(-child, libc::SIGKILL);
        libc::kill(child, libc::SIGKILL);
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn terminate_descendant_group(child: libc::pid_t) {
    unsafe {
        libc::kill(-child, libc::SIGKILL);
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn reap(child: libc::pid_t) {
    let mut status = 0;
    loop {
        let waited = unsafe { libc::waitpid(child, &mut status, 0) };
        if waited == child || (waited < 0 && last_errno() != Some(libc::EINTR)) {
            return;
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn close_fd(descriptor: libc::c_int) {
    if descriptor >= 0 {
        unsafe {
            libc::close(descriptor);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn last_errno() -> Option<i32> {
    std::io::Error::last_os_error().raw_os_error()
}
