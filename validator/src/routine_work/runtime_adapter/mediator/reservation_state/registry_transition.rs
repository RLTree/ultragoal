use super::binding::ReservationBinding;
use super::read_source_binding::MediatorRegistry;
use super::*;

pub(super) fn retain_non_durable(artifacts: &BTreeMap<String, String>) {
    registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .non_durable_authenticated_artifacts
        .extend(artifacts.clone());
}

pub(super) fn authenticates_non_durable(digest: &str, witness: &str) -> bool {
    registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .non_durable_authenticated_artifacts
        .get(digest)
        .is_some_and(|expected| expected == witness)
}

pub(super) fn record_non_durable_failure(
    state: &mut MediatorRegistry,
    binding: &ReservationBinding,
    evidence: &ReservationFailureEvidence,
) -> Result<(), RoutineError> {
    match state.failure_records.get(binding.recovery_marker()) {
        Some(existing) if existing != evidence => Err(mediator_error(
            "mediator-reservation-failure-evidence-conflict",
        )),
        Some(_) => Ok(()),
        None if state.failure_records.len() >= 4_096 => Err(mediator_error(
            "mediator-reservation-failure-evidence-capacity-exhausted",
        )),
        None => {
            state
                .failure_records
                .insert(binding.recovery_marker().clone(), evidence.clone());
            Ok(())
        }
    }
}
