pub(crate) struct ReservationObservation<'a> {
    pub(crate) expected_root: &'a crate::orchestration::Actor,
    pub(crate) binding: &'a crate::orchestration::Binding,
    pub(crate) workspace_identity: &'a str,
    pub(crate) current_identity: &'a str,
    pub(crate) snapshot: &'a super::super::ProductSnapshot,
    pub(crate) prior_identity: Option<&'a str>,
    pub(crate) last_event: Option<&'a crate::orchestration::OrchestrationEvent>,
}

impl ProductionRootAuthority {
    pub(crate) fn reconcile_observation(
        &self,
        permit: &RootPermit,
        observation: ReservationObservation<'_>,
    ) -> Result<PermitReplayState, ProductError> {
        self.authority.verify_observation(
            permit,
            observation.expected_root,
            observation.binding,
            observation.workspace_identity,
        )?;
        if observation.snapshot.binding != *observation.binding {
            return Err(ProductError::AuthorityInvalid);
        }
        let id = permit_id(permit)?;
        let state = classify_observation(
            observation.current_identity,
            observation.snapshot,
            observation.prior_identity,
            observation.last_event,
            permit,
        )?;
        self.ledger.reconcile(&id, state)?;
        Ok(state.into())
    }

    /// Reads the authenticated replay state without creating or changing it.
    pub fn replay_state(
        &self,
        permit: &RootPermit,
    ) -> Result<Option<PermitReplayState>, ProductError> {
        self.ledger
            .state(&permit_id(permit)?)
            .map(|state| state.map(Into::into))
    }
}

fn classify_observation(
    current_identity: &str,
    snapshot: &super::super::ProductSnapshot,
    prior_identity: Option<&str>,
    last_event: Option<&crate::orchestration::OrchestrationEvent>,
    permit: &RootPermit,
) -> Result<LedgerState, ProductError> {
    if current_identity == permit.journal_head_identity {
        return Ok(LedgerState::Refused);
    }
    let causally_bound = prior_identity == Some(permit.journal_head_identity.as_str());
    if !causally_bound {
        return Err(ProductError::AuthorityInvalid);
    }
    let applied = causally_bound
        && match permit.operation {
            RootOperation::Resume => {
                resume_event_matches(permit, last_event) && !snapshot.recovery.interrupted_root
            }
            RootOperation::Reconcile => {
                reconciliation_event_matches(permit, last_event)?
                    && permit.target.operation_id.as_ref().is_some_and(|id| {
                        snapshot.settled_operations.contains(id)
                            && !snapshot.recovery.ambiguous_operations.contains(id)
                    })
            }
            RootOperation::Recover => {
                permit.target.operation_id.as_deref()
                    == Some(snapshot.journal_head.last_event_id.as_str())
                    && permit.target.recovered_binding.as_ref() == Some(&snapshot.binding)
            }
        };
    if !applied {
        return Err(ProductError::AuthorityInvalid);
    }
    Ok(LedgerState::Committed)
}

fn reconciliation_event_matches(
    permit: &RootPermit,
    last_event: Option<&crate::orchestration::OrchestrationEvent>,
) -> Result<bool, ProductError> {
    let Some(event) = last_event else {
        return Ok(false);
    };
    let crate::orchestration::EventKind::EffectReconciled {
        lease_id,
        resolution,
    } = &event.event
    else {
        return Ok(false);
    };
    let PermitDecisionBinding::ReconcileEffect {
        effect_resolution_commitment_id,
    } = &permit.decision_binding
    else {
        return Ok(false);
    };
    Ok(event.actor.as_str() == permit.root_actor
        && event.binding == permit.binding
        && event.logical_tick == permit.issued_tick
        && permit.target.lease_id.as_ref() == Some(lease_id)
        && permit.target.operation_id.as_ref() == Some(&resolution.operation_id)
        && *effect_resolution_commitment_id
            == resolution.commitment_id().map_err(ProductError::from)?)
}

fn resume_event_matches(
    permit: &RootPermit,
    last_event: Option<&crate::orchestration::OrchestrationEvent>,
) -> bool {
    last_event.is_some_and(|event| {
        matches!(event.event, crate::orchestration::EventKind::RootRecovered)
            && event.actor.as_str() == permit.root_actor
            && event.binding == permit.binding
            && event.logical_tick == permit.issued_tick
    })
}

impl From<LedgerState> for PermitReplayState {
    fn from(value: LedgerState) -> Self {
        match value {
            LedgerState::Issued => Self::Issued,
            LedgerState::Reserved => Self::Reserved,
            LedgerState::Committed => Self::Committed,
            LedgerState::Refused => Self::Refused,
            LedgerState::Ambiguous => Self::Ambiguous,
        }
    }
}
