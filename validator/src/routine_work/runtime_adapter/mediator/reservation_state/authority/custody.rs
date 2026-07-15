use super::super::binding::ReservationBinding;
use super::super::read_source_binding::{MediatorRegistry, release_active};
use super::super::*;
use super::staged_custody::StagedCustody;
use super::transition_flag::TransitionFlag;

pub(super) struct AttemptCustody {
    started: TransitionFlag,
    settled: TransitionFlag,
    staged: StagedCustody,
}

impl AttemptCustody {
    pub(super) fn new() -> Self {
        Self {
            started: TransitionFlag::new(),
            settled: TransitionFlag::new(),
            staged: StagedCustody::new(),
        }
    }

    pub(super) fn is_started(&self) -> bool {
        self.started.is_set()
    }

    pub(super) fn terminal_is_authoritative(&self) -> bool {
        self.settled.is_set() && self.staged.is_empty()
    }

    pub(super) fn require_open(&self) -> Result<(), RoutineError> {
        if self.settled.is_set() {
            return Err(mediator_error(
                "mediator-reservation-terminal-already-settled",
            ));
        }
        Ok(())
    }

    pub(super) fn mark_started(&self, binding: &ReservationBinding) -> Result<(), RoutineError> {
        self.require_open()?;
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state
            .ambiguous_protocols
            .get(binding.protocol_id())
            .is_some_and(|actual| {
                actual != binding.recovery_marker()
                    && binding.prior_recovery_marker() != Some(actual)
            })
        {
            return Err(mediator_error("mediator-recovery-marker-conflict"));
        }
        state.ambiguous_protocols.insert(
            binding.protocol_id().clone(),
            binding.recovery_marker().clone(),
        );
        self.started.mark();
        Ok(())
    }

    pub(super) fn finish_terminal(
        &self,
        binding: &ReservationBinding,
        durable: bool,
    ) -> Option<String> {
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        release_active(&mut state, binding.protocol_id(), binding.grant_id());
        if durable {
            clear_exact_ambiguity(&mut state, binding, self.started.is_set());
            clear_exact_failure(&mut state, binding);
        }
        let pending = binding
            .expected_ambiguity(self.started.is_set())
            .filter(|expected| {
                state.ambiguous_protocols.get(binding.protocol_id()) == Some(*expected)
            })
            .cloned();
        self.settled.mark();
        pending
    }

    pub(super) fn require_staged_empty(&self) -> Result<(), RoutineError> {
        self.staged.require_empty()
    }

    pub(super) fn stage_and_use<T>(
        &self,
        staged: StagedProgram,
        use_program: impl FnOnce(&PinnedExecutable) -> Result<T, RoutineError>,
    ) -> Result<T, RoutineError> {
        self.staged.push_and_use(staged, use_program)
    }

    pub(super) fn cleanup_staged(
        &self,
        durable: bool,
        mut cleanup: impl FnMut(&StagedProgram) -> Result<(), RoutineError>,
    ) -> Result<(), RoutineError> {
        if !durable {
            return if self.staged.is_empty() {
                Ok(())
            } else {
                Err(mediator_error("mediator-staging-authority-missing"))
            };
        }
        while self.staged.cleanup_last(|staged| cleanup(staged))? {}
        Ok(())
    }

    pub(super) fn failure_transfer_required(
        &self,
        evidence: &CleanupEvidence,
        durable: bool,
    ) -> Result<bool, RoutineError> {
        self.staged.failure_transfer_required(evidence, durable)
    }

    pub(super) fn finish_failure(&self, transfer: bool, transition: impl FnOnce(bool)) {
        if transfer {
            self.staged.clear_recorded();
        }
        transition(self.started.is_set());
        self.settled.mark();
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
