use super::*;

#[cfg(unix)]
pub(crate) fn process_group_exists(group: ProcessGroupId) -> Result<bool, RoutineError> {
    if unsafe { libc::kill(group.signal_target()?, 0) } == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(libc::ESRCH) => Ok(false),
        Some(libc::EPERM) => Ok(true),
        _ => Err(mediator_error("mediator-process-group-probe-failed")),
    }
}

#[cfg(unix)]
pub(crate) fn signal_group(group: ProcessGroupId, signal: i32) -> Result<(), RoutineError> {
    if unsafe { libc::kill(group.signal_target()?, signal) } == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(mediator_error("mediator-process-group-signal-failed"))
    }
}

#[cfg(unix)]
pub(crate) fn cleanup_spawned_child(
    child: &mut BoundChild,
    process_group: Option<ProcessGroupId>,
) -> Result<(), RoutineError> {
    if let Some(process_group) = process_group {
        terminate_and_reap(child, process_group)
    } else {
        let kill = child.kill();
        let wait = child
            .wait()
            .map(|_| ())
            .map_err(|_| mediator_error("mediator-process-reap-failed"));
        match (normalize_kill(kill), wait) {
            (Err(error), _) | (Ok(()), Err(error)) => Err(error),
            (Ok(()), Ok(())) => Ok(()),
        }
    }
}

#[cfg(unix)]
pub(crate) fn terminate_and_reap(
    child: &mut BoundChild,
    group: ProcessGroupId,
) -> Result<(), RoutineError> {
    let mut first_error = None;
    if let Err(error) = signal_group(group, libc::SIGTERM) {
        first_error = Some(error);
    }
    let deadline = Instant::now() + Duration::from_millis(100);
    let mut parent_reaped = false;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => {
                parent_reaped = true;
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(2)),
            Err(_) => {
                first_error
                    .get_or_insert_with(|| mediator_error("mediator-process-reap-status-failed"));
                break;
            }
        }
    }
    if !parent_reaped {
        if let Err(error) = signal_group(group, libc::SIGKILL) {
            first_error.get_or_insert(error);
        }
        if let Err(error) = normalize_kill(child.kill()) {
            first_error.get_or_insert(error);
        }
        if child.wait().is_err() {
            first_error.get_or_insert_with(|| mediator_error("mediator-process-reap-failed"));
        }
    }
    match process_group_exists(group) {
        Ok(true) => {
            if let Err(error) = signal_group(group, libc::SIGKILL) {
                first_error.get_or_insert(error);
            }
        }
        Ok(false) => {}
        Err(error) => {
            first_error.get_or_insert(error);
            if let Err(error) = signal_group(group, libc::SIGKILL) {
                first_error.get_or_insert(error);
            }
        }
    }
    if let Err(error) = wait_group_absent(group) {
        first_error.get_or_insert(error);
    }
    first_error.map_or(Ok(()), Err)
}

fn normalize_kill(result: std::io::Result<()>) -> Result<(), RoutineError> {
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Ok(()),
        Err(_) => Err(mediator_error("mediator-process-signal-failed")),
    }
}

#[cfg(unix)]
pub(crate) fn terminate_group_after_parent_exit(group: ProcessGroupId) -> Result<(), RoutineError> {
    signal_group(group, libc::SIGTERM)?;
    let deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < deadline {
        if !process_group_exists(group)? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    signal_group(group, libc::SIGKILL)?;
    wait_group_absent(group)
}

#[cfg(unix)]
pub(crate) fn wait_group_absent(group: ProcessGroupId) -> Result<(), RoutineError> {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if !process_group_exists(group)? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err(mediator_error("mediator-descendant-cleanup-incomplete"))
}

#[cfg(test)]
mod tests {
    use super::super::ProcessGroupId;

    #[test]
    fn positive_pid_is_never_used_as_a_process_group_signal_target() {
        let group = ProcessGroupId::new(78_302).unwrap();
        assert_eq!(group.signal_target().unwrap(), -78_302);
        assert!(ProcessGroupId::new(0).is_err());
        assert!(ProcessGroupId::new(-1).is_err());
        assert!(ProcessGroupId::new(i32::MIN).is_err());
        assert!(ProcessGroupId::from_child_id(i32::MAX as u32 + 1).is_err());
    }
}
