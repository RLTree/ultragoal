use super::super::super::super::grant_validation::reservation_starts_fresh;
use super::super::super::super::*;
use super::super::binding::ReservationBinding;

#[derive(Default)]
struct RegistryState {
    consumed_grants: BTreeSet<String>,
    non_durable_artifacts: BTreeMap<String, String>,
    ambiguous_protocols: BTreeMap<String, String>,
    active_protocols: BTreeMap<String, String>,
    failure_records: BTreeMap<String, ReservationFailureEvidence>,
}

static REGISTRY: OnceLock<Mutex<RegistryState>> = OnceLock::new();

fn registry() -> &'static Mutex<RegistryState> {
    REGISTRY.get_or_init(|| Mutex::new(RegistryState::default()))
}

pub(super) fn reserve(grant: &RoutineRootGrant) -> Result<(), RoutineError> {
    let mut state = lock_registry();
    if state.consumed_grants.contains(&grant.grant_id) {
        return Err(mediator_error("mediator-root-grant-replayed"));
    }
    if state.active_protocols.contains_key(&grant.protocol_id) {
        return Err(mediator_error("mediator-protocol-attempt-active"));
    }
    if reservation_starts_fresh(state.ambiguous_protocols.get(&grant.protocol_id), grant)? {
        state
            .failure_records
            .retain(|_, record| record.protocol_id != grant.protocol_id);
    }
    state.consumed_grants.insert(grant.grant_id.clone());
    state
        .active_protocols
        .insert(grant.protocol_id.clone(), grant.grant_id.clone());
    Ok(())
}

pub(super) fn mark_started(binding: &ReservationBinding) -> Result<(), RoutineError> {
    let mut state = lock_registry();
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
    let mut state = lock_registry();
    release_active(&mut state, binding);
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
    lock_registry()
        .non_durable_artifacts
        .extend(artifacts.clone());
}

pub(super) fn authenticates_non_durable(digest: &str, witness: &str) -> bool {
    lock_registry()
        .non_durable_artifacts
        .get(digest)
        .is_some_and(|expected| expected == witness)
}

pub(super) fn record_failure_and_transition(
    binding: &ReservationBinding,
    evidence: &ReservationFailureEvidence,
    started: bool,
    record: impl FnOnce() -> Result<bool, RoutineError>,
) -> Result<(), RoutineError> {
    let mut state = lock_registry();
    if state.active_protocols.get(binding.protocol_id()) != Some(binding.grant_id()) {
        return Err(mediator_error(
            "mediator-reservation-failure-active-binding-invalid",
        ));
    }
    if !record()? {
        record_non_durable_failure(&mut state, binding, evidence)?;
    }
    release_active(&mut state, binding);
    if started
        && !state
            .ambiguous_protocols
            .contains_key(binding.protocol_id())
    {
        state.ambiguous_protocols.insert(
            binding.protocol_id().clone(),
            binding.recovery_marker().clone(),
        );
    }
    Ok(())
}

fn lock_registry() -> std::sync::MutexGuard<'static, RegistryState> {
    registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn release_active(state: &mut RegistryState, binding: &ReservationBinding) {
    if state.active_protocols.get(binding.protocol_id()) == Some(binding.grant_id()) {
        state.active_protocols.remove(binding.protocol_id());
    }
}

fn record_non_durable_failure(
    state: &mut RegistryState,
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

fn clear_exact_ambiguity(state: &mut RegistryState, binding: &ReservationBinding, started: bool) {
    if binding.expected_ambiguity(started).is_some_and(|expected| {
        state.ambiguous_protocols.get(binding.protocol_id()) == Some(expected)
    }) {
        state.ambiguous_protocols.remove(binding.protocol_id());
    }
}

fn clear_exact_failure(state: &mut RegistryState, binding: &ReservationBinding) {
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

#[cfg(test)]
pub(in super::super::super::super) fn observe(
    protocol: &str,
    failure_marker: Option<&str>,
    artifact_digest: Option<&str>,
) -> super::super::ReservationObservation {
    let state = lock_registry();
    super::super::ReservationObservation {
        active_grant: state.active_protocols.get(protocol).cloned(),
        recovery_marker: state.ambiguous_protocols.get(protocol).cloned(),
        failure: failure_marker.and_then(|marker| state.failure_records.get(marker).cloned()),
        non_durable_witness: artifact_digest
            .and_then(|digest| state.non_durable_artifacts.get(digest).cloned()),
    }
}
