use super::*;

#[derive(Default)]
pub(super) struct TerminalDurable {
    pub(super) settlements: Mutex<Vec<DurableSettlement>>,
}

impl DurableAttemptAuthority for TerminalDurable {
    fn validate_reserved(&self) -> Result<(), RoutineError> {
        Ok(())
    }

    fn stage_program(&self, _program: &PinnedExecutable) -> Result<StagedProgram, RoutineError> {
        Err(mediator_error("terminal-settlement-test-stage-unavailable"))
    }

    fn cleanup_staged(&self, _staged: &StagedProgram) -> Result<(), RoutineError> {
        Ok(())
    }

    fn prepare_spawn(&self) -> Result<(), RoutineError> {
        Ok(())
    }

    fn stage_success(&self, _artifacts: &BTreeMap<String, String>) -> Result<(), RoutineError> {
        Ok(())
    }

    fn settle(
        &self,
        outcome: DurableSettlement,
        _artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.settlements
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(outcome);
        Ok(())
    }

    fn authenticates_artifact(&self, _digest: &str, _witness: &str) -> Result<bool, RoutineError> {
        Ok(false)
    }

    fn recovery_is_durable(&self) -> bool {
        true
    }

    fn reuse_only(&self) -> bool {
        false
    }
}

pub(super) fn attempt(
    label: &str,
    durable: Option<Arc<dyn DurableAttemptAuthority>>,
    started: bool,
    prior_recovery_marker: Option<String>,
) -> AttemptReservation {
    AttemptReservation {
        protocol_id: format!("terminal-protocol-{label}"),
        grant_id: format!("terminal-grant-{label}"),
        recovery_marker: format!("terminal-marker-{label}"),
        prior_recovery_marker,
        started: Cell::new(started),
        settled: Cell::new(false),
        durable,
        staged: RefCell::new(Vec::new()),
    }
}

pub(super) fn seed(attempt: &AttemptReservation, active_grant: &str, ambiguity: &str) {
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state
        .active_protocols
        .insert(attempt.protocol_id.clone(), active_grant.to_owned());
    state
        .ambiguous_protocols
        .insert(attempt.protocol_id.clone(), ambiguity.to_owned());
}

pub(super) fn retry_grant(
    reservation: &AttemptReservation,
    durable: Arc<dyn DurableAttemptAuthority>,
) -> RoutineRootGrant {
    RoutineRootGrant {
        grant_id: format!("{}-retry", reservation.grant_id),
        session_id: "terminal-session-retry".to_owned(),
        request_id: "terminal-request-retry".to_owned(),
        protocol_id: reservation.protocol_id.clone(),
        context_id: "terminal-context-retry".to_owned(),
        candidate_id: "terminal-candidate-retry".to_owned(),
        plan_id: "terminal-plan-retry".to_owned(),
        snapshot_id: "terminal-snapshot-retry".to_owned(),
        allowed_output_scopes: Vec::new(),
        recovery_for: None,
        seal: "terminal-seal-retry".to_owned(),
        durable: Some(durable),
    }
}
