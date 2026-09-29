use std::io;
use std::time::{Duration, Instant};

pub(crate) fn cleanup_process(
    pid: libc::pid_t,
    group: libc::pid_t,
    reaped: bool,
) -> io::Result<()> {
    if reaped {
        return group_exists(group)
            .then(|| {
                io::Error::other(
                    "darwin descendant cleanup refused after leader identity was reaped",
                )
            })
            .map_or(Ok(()), Err);
    }
    require_live_leader(reaped)?;
    let mut failure = None;
    if let Err(error) = signal_group(group, libc::SIGTERM) {
        failure = Some(error);
    }
    let child_reaped = match wait_child_bounded(pid, Duration::from_millis(100)) {
        Ok(reaped) => reaped,
        Err(error) => {
            failure.get_or_insert(error);
            false
        }
    };
    if child_reaped {
        if group_exists(group) {
            failure.get_or_insert_with(|| {
                io::Error::other(
                    "darwin descendant cleanup refused after leader identity was reaped",
                )
            });
        }
        return failure.map_or(Ok(()), Err);
    }
    if group_exists(group) {
        require_live_leader(child_reaped)?;
        if let Err(error) = signal_group(group, libc::SIGKILL) {
            failure.get_or_insert(error);
        }
    }
    let child_reaped = match wait_child_bounded(pid, Duration::from_millis(500)) {
        Ok(reaped) => reaped,
        Err(error) => {
            failure.get_or_insert(error);
            false
        }
    };
    if !child_reaped {
        return Err(io::Error::other("darwin child reap timed out"));
    }
    if group_exists(group) {
        failure.get_or_insert_with(|| {
            io::Error::other("darwin descendant cleanup incomplete after leader reap")
        });
    }
    failure.map_or(Ok(()), Err)
}

fn require_live_leader(reaped: bool) -> io::Result<()> {
    if reaped {
        Err(io::Error::other(
            "darwin group signal refused without live leader identity",
        ))
    } else {
        Ok(())
    }
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

#[cfg(test)]
pub(crate) fn wait_group_absent(group: libc::pid_t) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < deadline {
        if !group_exists(group) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err(io::Error::other(
        "darwin descendant cleanup did not become absent",
    ))
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

#[cfg(test)]
mod tests {
    use super::require_live_leader;

    #[test]
    fn numeric_group_signal_requires_a_live_leader_identity() {
        require_live_leader(false).expect("live leader anchors process group identity");
        let error = require_live_leader(true).expect_err("reaped leader must close signaling");
        assert!(error.to_string().contains("without live leader identity"));
    }
}
