use super::*;

pub(crate) fn process_group_exists(group: ProcessGroupId) -> Result<bool, RoutineError> {
    Ok(crate::process_custody::darwin_cleanup::group_exists(
        group.raw(),
    ))
}

pub(crate) fn signal_group(group: ProcessGroupId, signal: i32) -> Result<(), RoutineError> {
    crate::process_custody::darwin_cleanup::signal_group(group.raw(), signal)
        .map_err(|_| mediator_error("mediator-process-group-signal-failed"))
}

pub(crate) fn wait_group_absent(group: ProcessGroupId) -> Result<(), RoutineError> {
    crate::process_custody::darwin_cleanup::wait_group_absent(group.raw())
        .map_err(|_| mediator_error("mediator-descendant-cleanup-incomplete"))
}
