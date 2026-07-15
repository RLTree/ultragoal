use super::binding::ReservationBinding;
use super::read_source_binding::{MediatorRegistry, release_active};
use super::*;

pub(super) fn mark_started(binding: &ReservationBinding) -> Result<(), RoutineError> {
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state
        .ambiguous_protocols
        .get(binding.protocol_id())
        .is_some_and(|actual| {
            actual != binding.recovery_marker() && binding.prior_recovery_marker() != Some(actual)
        })
    {
        return Err(mediator_error("mediator-recovery-marker-conflict"));
    }
    state.ambiguous_protocols.insert(
        binding.protocol_id().clone(),
        binding.recovery_marker().clone(),
    );
    Ok(())
}

pub(super) fn finish_terminal(
    binding: &ReservationBinding,
    started: bool,
    durable: bool,
) -> Option<String> {
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    release_active(&mut state, binding.protocol_id(), binding.grant_id());
    if durable {
        clear_exact_ambiguity(&mut state, binding, started);
        clear_exact_failure(&mut state, binding);
    }
    binding
        .expected_ambiguity(started)
        .filter(|expected| state.ambiguous_protocols.get(binding.protocol_id()) == Some(*expected))
        .cloned()
}

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

fn clear_exact_ambiguity(
    state: &mut MediatorRegistry,
    binding: &ReservationBinding,
    started: bool,
) {
    if binding.expected_ambiguity(started).is_some_and(|expected| {
        state.ambiguous_protocols.get(binding.protocol_id()) == Some(expected)
    }) {
        state.ambiguous_protocols.remove(binding.protocol_id());
    }
}

fn clear_exact_failure(state: &mut MediatorRegistry, binding: &ReservationBinding) {
    for marker in [
        Some(binding.recovery_marker().as_str()),
        binding.prior_recovery_marker().map(String::as_str),
    ]
    .into_iter()
    .flatten()
    {
        if state.failure_records.get(marker).is_some_and(|record| {
            record.protocol_id == *binding.protocol_id() && record.recovery_marker == marker
        }) {
            state.failure_records.remove(marker);
        }
    }
}
