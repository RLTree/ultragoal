use super::*;

/// Refuses public routine entry before discovery, tool probes, dirty capture,
/// host state, output provisioning, or durable reservation on the supported
/// host until a root-owned broker is wired.
pub(crate) fn authorize() -> Result<(), PublicFailure> {
    crate::routine_work::require_root_broker_for_public_effect().map_err(PublicFailure::Routine)
}
