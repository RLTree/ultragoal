use std::io;
use std::time::{Duration, Instant};

pub(super) fn settle(pid: libc::pid_t, reaped: bool) -> bool {
    let mut child_reaped = reaped;
    if !child_reaped {
        terminate_group(pid);
        child_reaped = wait_child(pid, Duration::from_millis(500));
    }
    if group_exists(pid) {
        terminate_group(pid);
    }
    child_reaped && wait_group_absent(pid)
}

pub(super) fn group_exists(pid: libc::pid_t) -> bool {
    // SAFETY: signal zero probes only the captured positive process group.
    let result = unsafe { libc::kill(-pid, 0) };
    result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn terminate_group(pid: libc::pid_t) {
    // SAFETY: pid is the direct child and therefore the captured group id.
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
        libc::kill(pid, libc::SIGKILL);
    }
}

fn wait_child(pid: libc::pid_t, budget: Duration) -> bool {
    let deadline = Instant::now() + budget;
    let mut status = 0;
    while Instant::now() < deadline {
        // SAFETY: pid is the direct child returned by posix_spawn and status is writable.
        let result = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if result == pid {
            return true;
        }
        if result < 0 {
            return io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    false
}

fn wait_group_absent(pid: libc::pid_t) -> bool {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if !group_exists(pid) {
            return true;
        }
        terminate_group(pid);
        std::thread::sleep(Duration::from_millis(2));
    }
    !group_exists(pid)
}
