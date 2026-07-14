use super::*;

impl RepositoryFitProductionOutcome {
    pub(crate) fn result_id(&self) -> &str {
        &self.result_id
    }

    pub(crate) fn status(&self) -> &str {
        self.status
    }

    pub(crate) const fn error_id(&self) -> Option<AdapterErrorId> {
        self.adapter_error_id
    }

    pub(crate) const fn effect_started(&self) -> bool {
        self.effect_started
    }

    pub(crate) const fn rollback_complete(&self) -> bool {
        self.rollback_complete
    }

    /// A durable recovery envelope must be retained only while the ledger has
    /// not reached a terminal state. Callers must not infer this from status
    /// prose or from whether an effect was observed.
    pub(crate) fn recovery_required(&self) -> bool {
        matches!(self.ledger_state, "reserved" | "effect_started")
    }

    pub(crate) fn to_machine_bytes(&self) -> Result<Vec<u8>, FitAdapterError> {
        let bytes = serde_json::to_vec(self)
            .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?;
        if bytes.len() > MAX_OUTCOME_BYTES {
            return Err(adapter_error(AdapterErrorId::ProjectionFailed));
        }
        Ok(bytes)
    }

    pub(crate) fn refusal(request_id: String, error: FitAdapterError) -> Self {
        Self::new(
            request_id,
            "refused",
            Some(error.id()),
            "absent",
            false,
            false,
            None,
            "none",
        )
    }

    pub(crate) fn causal_refusal(
        request_id: String,
        error: FitAdapterError,
        state: RepositoryFitLedgerState,
        effect_started: bool,
    ) -> Self {
        Self::new(
            request_id,
            "refused",
            Some(error.id()),
            state.name(),
            effect_started,
            false,
            None,
            "none",
        )
    }

    pub(crate) fn terminal_failure(
        request_id: String,
        error: FitAdapterError,
        state: RepositoryFitLedgerState,
        effect_started: bool,
        rollback_complete: bool,
    ) -> Self {
        let status = match state {
            RepositoryFitLedgerState::RolledBack => "rolled_back",
            RepositoryFitLedgerState::Interrupted => "interrupted",
            RepositoryFitLedgerState::Ambiguous => "ambiguous",
            RepositoryFitLedgerState::Rejected => "refused",
            RepositoryFitLedgerState::Reserved
            | RepositoryFitLedgerState::EffectStarted
            | RepositoryFitLedgerState::Committed => "ambiguous",
        };
        Self::new(
            request_id,
            status,
            Some(error.id()),
            state.name(),
            effect_started,
            rollback_complete,
            None,
            if effect_started {
                "workspace_write"
            } else {
                "none"
            },
        )
    }

    pub(crate) fn success(request_id: String, apply: RepositoryFitApplyOutcome) -> Self {
        let status = if apply.mutation_count() == 0 {
            "idempotent"
        } else {
            "applied"
        };
        Self::new(
            request_id,
            status,
            None,
            RepositoryFitLedgerState::Committed.name(),
            true,
            false,
            Some(apply),
            "workspace_write",
        )
    }

    pub(crate) fn recovered(
        request_id: String,
        state: RepositoryFitLedgerState,
        effect_started: bool,
    ) -> Self {
        let (status, error_id) = match state {
            RepositoryFitLedgerState::Committed => ("recovered", None),
            RepositoryFitLedgerState::Interrupted => {
                ("interrupted", Some(AdapterErrorId::ApplyOutcomeAmbiguous))
            }
            RepositoryFitLedgerState::Ambiguous => {
                ("ambiguous", Some(AdapterErrorId::ApplyOutcomeAmbiguous))
            }
            _ => ("ambiguous", Some(AdapterErrorId::ApplyOutcomeInvalid)),
        };
        Self::new(
            request_id,
            status,
            error_id,
            state.name(),
            effect_started,
            false,
            None,
            "none",
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        request_id: String,
        status: &'static str,
        adapter_error_id: Option<AdapterErrorId>,
        ledger_state: &'static str,
        effect_started: bool,
        rollback_complete: bool,
        apply_outcome: Option<RepositoryFitApplyOutcome>,
        effect: &'static str,
    ) -> Self {
        let result_id = digest(
            &serde_json::to_vec(&(
                "repository-fit-production-outcome-v1",
                &request_id,
                status,
                adapter_error_id,
                ledger_state,
                effect_started,
                rollback_complete,
                apply_outcome
                    .as_ref()
                    .map(RepositoryFitApplyOutcome::outcome_id),
                effect,
                "none",
                SUPPORT_LIMIT,
            ))
            .expect("fixed repository-fit production outcome is serializable"),
        );
        Self {
            schema_version: "RepositoryFitProductionOutcome-v1",
            result_id,
            request_id,
            status,
            adapter_error_id,
            ledger_state,
            effect_started,
            rollback_complete,
            apply_outcome,
            effect,
            claim_effect: "none",
            support_limit: SUPPORT_LIMIT,
        }
    }
}
