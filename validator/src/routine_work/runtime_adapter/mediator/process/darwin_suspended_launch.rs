use super::*;
use std::collections::BTreeMap;

pub(crate) struct SpawnedProcess {
    pub(crate) process: crate::process_custody::DarwinSuspendedProcess,
}

pub(crate) fn spawn_suspended(
    program: &PinnedExecutable,
    root: &RootAnchor,
    argv: &[String],
    environment: &BTreeMap<String, String>,
) -> Result<SpawnedProcess, RoutineError> {
    let spawned = crate::process_custody::spawn_suspended_descriptor(
        program.path(),
        root.raw_fd(),
        argv,
        environment,
    )
    .map_err(|_| mediator_error("mediator-process-launch-failed"))?;
    Ok(SpawnedProcess { process: spawned })
}
