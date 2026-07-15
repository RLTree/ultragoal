use super::read_source_binding::{release_active, MediatorRegistry};
use super::*;
use std::cell::{Cell, RefCell};

#[path = "artifact_authentication.rs"]
mod artifact_authentication;
#[path = "failure_transition.rs"]
mod failure_transition;
#[path = "lifecycle.rs"]
mod lifecycle;
#[path = "staged_custody.rs"]
mod staged_custody;
#[cfg(test)]
#[path = "terminal_custody_tests.rs"]
mod terminal_custody_tests;

pub(super) use lifecycle::{observe_staged_transition, run_reserved};

/// Process-local attempt reservation guarding the gap between grant
/// consumption and final reconciliation.
///
/// Only these checked transitions may mutate lifecycle or staged custody.
/// Leaving scope never changes authority state.
pub(crate) struct AttemptReservation {
    protocol_id: String,
    grant_id: String,
    recovery_marker: String,
    prior_recovery_marker: Option<String>,
    started: Cell<bool>,
    settled: Cell<bool>,
    durable: Option<Arc<dyn DurableAttemptAuthority>>,
    staged: RefCell<Vec<StagedProgram>>,
}

impl AttemptReservation {
    pub(super) fn reserved(
        protocol_id: String,
        grant_id: String,
        recovery_marker: String,
        prior_recovery_marker: Option<String>,
        durable: Option<Arc<dyn DurableAttemptAuthority>>,
    ) -> Self {
        Self {
            protocol_id,
            grant_id,
            recovery_marker,
            prior_recovery_marker,
            started: Cell::new(false),
            settled: Cell::new(false),
            durable,
            staged: RefCell::new(Vec::new()),
        }
    }

    #[cfg(test)]
    pub(super) fn reserved_test_attempt(
        protocol_id: String,
        grant_id: String,
        recovery_marker: String,
        prior_recovery_marker: Option<String>,
        durable: Option<Arc<dyn DurableAttemptAuthority>>,
        started: bool,
    ) -> Self {
        let attempt = Self::reserved(
            protocol_id,
            grant_id,
            recovery_marker,
            prior_recovery_marker,
            durable,
        );
        attempt.started.set(started);
        attempt
    }

    #[cfg(test)]
    pub(super) fn protocol_id(&self) -> &String {
        &self.protocol_id
    }

    #[cfg(test)]
    pub(super) fn grant_id(&self) -> &String {
        &self.grant_id
    }

    #[cfg(test)]
    pub(super) fn recovery_marker(&self) -> &String {
        &self.recovery_marker
    }

    pub(super) fn is_started(&self) -> bool {
        self.started.get()
    }

    pub(super) fn terminal_is_authoritative(&self) -> bool {
        self.settled.get() && !self.has_staged_custody()
    }

    fn require_open(&self) -> Result<(), RoutineError> {
        if self.settled.get() {
            return Err(mediator_error(
                "mediator-reservation-terminal-already-settled",
            ));
        }
        Ok(())
    }

    fn expected_ambiguity(&self) -> Option<&String> {
        if self.started.get() {
            Some(&self.recovery_marker)
        } else {
            self.prior_recovery_marker.as_ref()
        }
    }

    fn clear_exact_ambiguity(&self, state: &mut MediatorRegistry) {
        if self.expected_ambiguity().is_some_and(|expected| {
            state.ambiguous_protocols.get(&self.protocol_id) == Some(expected)
        }) {
            state.ambiguous_protocols.remove(&self.protocol_id);
        }
    }

    pub(super) fn reuse_only(&self) -> bool {
        self.durable
            .as_ref()
            .is_some_and(|durable| durable.reuse_only())
    }

    pub(super) fn prepare_spawn(&self) -> Result<(), RoutineError> {
        self.require_open()?;
        if let Some(durable) = &self.durable {
            durable.prepare_spawn()?;
        }
        Ok(())
    }

    pub(super) fn mark_started(&self) -> Result<(), RoutineError> {
        self.require_open()?;
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state
            .ambiguous_protocols
            .get(&self.protocol_id)
            .is_some_and(|marker| marker != &self.recovery_marker)
        {
            return Err(mediator_error("mediator-recovery-marker-conflict"));
        }
        state
            .ambiguous_protocols
            .insert(self.protocol_id.clone(), self.recovery_marker.clone());
        self.started.set(true);
        Ok(())
    }

    pub(super) fn stage_success(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.require_open()?;
        self.require_staged_empty()?;
        if let Some(durable) = &self.durable {
            durable.stage_success(artifacts)?;
        }
        Ok(())
    }

    pub(super) fn settle_success(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.require_open()?;
        self.require_staged_empty()?;
        if let Some(durable) = &self.durable {
            durable.settle(DurableSettlement::Complete, artifacts)?;
        }
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        release_active(&mut state, &self.protocol_id, &self.grant_id);
        self.clear_exact_ambiguity(&mut state);
        self.clear_exact_failure(&mut state);
        self.settled.set(true);
        Ok(())
    }

    pub(super) fn settle_incomplete(
        &self,
        outcome: DurableSettlement,
    ) -> Result<Option<String>, RoutineError> {
        self.require_open()?;
        self.require_staged_empty()?;
        let durably_terminal = if let Some(durable) = &self.durable {
            durable.settle(outcome, &BTreeMap::new())?;
            true
        } else {
            false
        };
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        release_active(&mut state, &self.protocol_id, &self.grant_id);
        if durably_terminal {
            self.clear_exact_ambiguity(&mut state);
            self.clear_exact_failure(&mut state);
        }
        let pending_marker = (!durably_terminal)
            .then(|| {
                self.expected_ambiguity()
                    .filter(|expected| {
                        state.ambiguous_protocols.get(&self.protocol_id) == Some(*expected)
                    })
                    .cloned()
            })
            .flatten();
        self.settled.set(true);
        Ok(pending_marker)
    }
}
