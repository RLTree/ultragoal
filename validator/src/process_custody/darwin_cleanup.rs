use std::io;
use std::time::{Duration, Instant};

pub(crate) fn cleanup_process(
    pid: libc::pid_t,
    group: libc::pid_t,
    reaped: bool,
) -> io::Result<()> {
    let mut child_reaped = reaped;
    let mut failure = None;
    if !child_reaped {
        if let Err(error) = signal_group(group, libc::SIGTERM) {
            failure = Some(error);
        }
        child_reaped = match wait_child_bounded(pid, Duration::from_millis(100)) {
            Ok(reaped) => reaped,
            Err(error) => {
                failure.get_or_insert(error);
                false
            }
        };
    }
    if group_exists(group) {
        if let Err(error) = signal_group(group, libc::SIGKILL) {
            failure.get_or_insert(error);
        }
    }
    if !child_reaped {
        child_reaped = match wait_child_bounded(pid, Duration::from_millis(500)) {
            Ok(reaped) => reaped,
            Err(error) => {
                failure.get_or_insert(error);
                false
            }
        };
    }
    if !child_reaped {
        return Err(io::Error::other("darwin child reap timed out"));
    }
    if let Err(error) = wait_group_absent(group) {
        failure.get_or_insert(error);
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn group_exists(group: libc::pid_t) -> bool {
    // SAFETY: signal zero probes only the captured process group.
    let result = unsafe { libc::kill(-group, 0) };
    result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

pub(crate) fn signal_group(group: libc::pid_t, signal: i32) -> io::Result<()> {
    // SAFETY: callers pass the captured positive process-group identity.
    if unsafe { libc::kill(-group, signal) } == 0 {
        Ok(())
    } else if io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub(crate) fn wait_group_absent(group: libc::pid_t) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if !group_exists(group) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err(io::Error::other("darwin descendant cleanup incomplete"))
}

fn wait_child_bounded(pid: libc::pid_t, budget: Duration) -> io::Result<bool> {
    let deadline = Instant::now() + budget;
    let mut status = 0;
    while Instant::now() < deadline {
        // SAFETY: pid is the direct child returned by posix_spawn and status is writable.
        let result = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if result == pid {
            return Ok(true);
        }
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            if error.raw_os_error() == Some(libc::ECHILD) {
                return Ok(true);
            }
            return Err(error);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(false)
}
