use super::*;
use std::convert::Infallible;

/// Opaque consumption boundary for a future root-owned broker adapter.
///
/// The uninhabited private field makes this value impossible to construct from
/// Routine-local safe code. Root integration must replace this refusal boundary,
/// not add a local issuer or serialized representation.
pub(crate) struct RootBrokerAuthorization {
    _root_owned: Infallible,
}

/// Refuses locally mediated execution before any child can be spawned.
///
/// A future root-owned broker adapter must replace this boundary with an
/// opaque authorization that routine-local code cannot construct or deserialize.
pub(crate) fn require_root_broker_before_spawn() -> Result<RootBrokerAuthorization, RoutineError> {
    Err(RoutineError::new(
        RoutineErrorId::InvalidRequest,
        "mediator-child-root-broker-required",
        None,
    ))
}

/// Refuses the public effect path before host state, output provisioning, or
/// durable reservation until a root-owned broker supplies this authority.
pub(crate) fn require_root_broker_for_public_effect() -> Result<(), RoutineError> {
    require_root_broker_before_spawn().map(drop)
}

pub(crate) fn spawn_with_root_broker(
    _authorization: &RootBrokerAuthorization,
    command: &mut Command,
) -> std::io::Result<Child> {
    command.spawn()
}

#[cfg(test)]
pub(crate) fn test_require_root_broker_before_spawn() -> Result<(), RoutineError> {
    require_root_broker_before_spawn().map(|_| ())
}
