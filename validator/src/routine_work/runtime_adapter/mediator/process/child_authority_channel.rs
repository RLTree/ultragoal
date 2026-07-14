use super::*;

/// Refuses locally mediated execution before any child can be spawned.
///
/// A future root-owned broker adapter must replace this boundary with an
/// opaque authorization that N06 code cannot construct or deserialize.
pub(crate) fn require_root_broker_before_spawn() -> Result<(), RoutineError> {
    Err(mediator_error("mediator-child-root-broker-required"))
}

#[cfg(test)]
pub(crate) fn test_require_root_broker_before_spawn() -> Result<(), RoutineError> {
    require_root_broker_before_spawn()
}
