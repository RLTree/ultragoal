#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use crate::distribution::host_effect::executor::{BackendFailure, HostEffectExecutorErrorId};

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(super) fn create_pipe() -> Result<(libc::c_int, libc::c_int), BackendFailure> {
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
pub(super) fn set_nonblocking(descriptor: libc::c_int) -> Result<(), ()> {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } != 0
    {
        return Err(());
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(super) fn drain(
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
pub(super) fn terminate_process_group(child: libc::pid_t) {
    unsafe {
        libc::kill(-child, libc::SIGKILL);
        libc::kill(child, libc::SIGKILL);
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(super) fn terminate_descendant_group(child: libc::pid_t) {
    unsafe {
        libc::kill(-child, libc::SIGKILL);
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(super) fn reap(child: libc::pid_t) {
    let mut status = 0;
    loop {
        let waited = unsafe { libc::waitpid(child, &mut status, 0) };
        if waited == child || (waited < 0 && last_errno() != Some(libc::EINTR)) {
            return;
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(super) fn close_fd(descriptor: libc::c_int) {
    if descriptor >= 0 {
        unsafe {
            libc::close(descriptor);
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(super) fn last_errno() -> Option<i32> {
    std::io::Error::last_os_error().raw_os_error()
}
