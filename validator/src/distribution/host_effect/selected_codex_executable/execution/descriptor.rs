use super::super::SelectedCodexExecutable;
use super::process_group::{
    close_fd, create_pipe, drain, last_errno, reap, set_nonblocking, terminate_descendant_group,
    terminate_process_group,
};
use crate::distribution::HostCommand;
use crate::distribution::host_effect::executor::{
    BackendFailure, CommandCapture, HostEffectCancellation, HostEffectExecutionPolicy,
    HostEffectExecutorErrorId,
};
use crate::distribution::host_effect::lifecycle::DescriptorExecutionPrimitive;

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(super) fn execute(
    executable: &SelectedCodexExecutable,
    command: &HostCommand,
    policy: &HostEffectExecutionPolicy,
    cancellation: &HostEffectCancellation,
    environment_entries: &[(String, String)],
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
    let environment_values = environment_entries
        .iter()
        .map(|(key, value)| CString::new(format!("{key}={value}")))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            BackendFailure::before_start(HostEffectExecutorErrorId::EnvironmentInjection)
        })?;
    let mut environment = environment_values
        .iter()
        .map(|value| value.as_ptr())
        .collect::<Vec<_>>();
    environment.push(std::ptr::null());

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
                        executable.raw_file().as_raw_fd(),
                        c"".as_ptr(),
                        argv.as_ptr(),
                        environment.as_ptr(),
                        libc::AT_EMPTY_PATH,
                    );
                }
                #[cfg(target_os = "freebsd")]
                DescriptorExecutionPrimitive::Fexecve => {
                    libc::fexecve(
                        executable.raw_file().as_raw_fd(),
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
