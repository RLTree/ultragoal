use super::*;

impl RoutineEffectRequest {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        request_id: String,
        protocol_id: String,
        binding: RoutineBinding,
        graph_id: String,
        snapshot_id: String,
        plan_id: String,
        result_scope: String,
        intents: Vec<RoutineEffectIntent>,
        issuance: u64,
        seal_id: String,
    ) -> Self {
        Self {
            request_id,
            protocol_id,
            binding,
            graph_id,
            snapshot_id,
            plan_id,
            result_scope,
            intents,
            seal: Arc::new(RequestSeal::new(issuance, seal_id)),
        }
    }

    pub(crate) fn request_id(&self) -> &str {
        &self.request_id
    }
    pub(crate) fn protocol_id(&self) -> &str {
        &self.protocol_id
    }
    pub(crate) fn context_id(&self) -> &str {
        self.binding.context_id()
    }
    pub(crate) fn candidate_id(&self) -> &str {
        self.binding.candidate_id()
    }
    pub(crate) fn plan_id(&self) -> &str {
        &self.plan_id
    }
    pub(crate) fn result_scope(&self) -> &str {
        &self.result_scope
    }
    pub(crate) fn intents(&self) -> &[RoutineEffectIntent] {
        &self.intents
    }

    pub(crate) fn seal_issuance(&self) -> u64 {
        self.seal.issuance()
    }

    pub(crate) fn seal_matches(&self, expected: &str) -> bool {
        self.seal.matches(expected)
    }

    pub(crate) fn begin_mediation(&self) -> Result<(), RoutineError> {
        self.seal.begin_mediation()
    }

    #[cfg(test)]
    pub(crate) fn test_mark_transitioned(&self) -> Result<(), RoutineError> {
        self.seal.begin_mediation()
    }

    #[cfg(test)]
    pub(crate) fn test_duplicate(&self) -> Self {
        Self {
            request_id: self.request_id.clone(),
            protocol_id: self.protocol_id.clone(),
            binding: self.binding.clone(),
            graph_id: self.graph_id.clone(),
            snapshot_id: self.snapshot_id.clone(),
            plan_id: self.plan_id.clone(),
            result_scope: self.result_scope.clone(),
            intents: self.intents.clone(),
            seal: Arc::clone(&self.seal),
        }
    }
}

#[must_use = "a mediation batch must be split and reconciled once"]
pub(crate) struct RoutineMediationBatch {
    pub(crate) authority: RoutineMediationAuthority,
    pub(crate) intents: Vec<RoutineMediatedIntent>,
}

impl RoutineMediationBatch {
    pub(crate) fn new(
        authority: RoutineMediationAuthority,
        intents: Vec<RoutineMediatedIntent>,
    ) -> Self {
        Self { authority, intents }
    }

    pub(crate) fn into_parts(self) -> (RoutineMediationAuthority, Vec<RoutineMediatedIntent>) {
        (self.authority, self.intents)
    }
}

pub(crate) struct MediatedExpectedRow {
    pub(crate) intent_id: String,
    pub(crate) plan_order: usize,
    pub(crate) node_id: String,
}

/// Opaque one-use authority for exactly one request issuance.
#[must_use = "mediation authority must be reconciled once or explicitly discarded"]
pub(crate) struct RoutineMediationAuthority {
    pub(crate) request_id: String,
    pub(crate) protocol_id: String,
    pub(crate) binding: RoutineBinding,
    pub(crate) graph_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) plan_id: String,
    pub(crate) requested_result_scope: String,
    pub(crate) execution_result_scope: String,
    pub(crate) expected: Vec<MediatedExpectedRow>,
    pub(crate) seal: Arc<RequestSeal>,
}

impl RoutineMediationAuthority {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        request_id: String,
        protocol_id: String,
        binding: RoutineBinding,
        graph_id: String,
        snapshot_id: String,
        plan_id: String,
        requested_result_scope: String,
        execution_result_scope: String,
        expected: Vec<MediatedExpectedRow>,
        seal: Arc<RequestSeal>,
    ) -> Self {
        Self {
            request_id,
            protocol_id,
            binding,
            graph_id,
            snapshot_id,
            plan_id,
            requested_result_scope,
            execution_result_scope,
            expected,
            seal,
        }
    }

    pub(crate) fn request_id(&self) -> &str {
        &self.request_id
    }

    pub(crate) fn protocol_id(&self) -> &str {
        &self.protocol_id
    }

    pub(crate) fn requested_result_scope(&self) -> &str {
        &self.requested_result_scope
    }

    pub(crate) fn execution_result_scope(&self) -> &str {
        &self.execution_result_scope
    }

    pub(crate) fn same_issuance(&self, outcome: &RoutineMediatedOutcome) -> bool {
        Arc::ptr_eq(&self.seal, &outcome.seal)
    }

    pub(crate) fn require_complete(&self) -> Result<(), RoutineError> {
        self.seal.require_complete(self.expected.len())
    }

    pub(crate) fn finish(&self) -> Result<(), RoutineError> {
        self.seal.finish(self.expected.len())
    }
}

/// Opaque one-use intent token issued by one mediation transition.
#[must_use = "a mediated intent must produce one observed outcome or be discarded"]
pub(crate) struct RoutineMediatedIntent {
    pub(crate) request_id: String,
    pub(crate) protocol_id: String,
    pub(crate) execution_result_scope: String,
    pub(crate) intent: RoutineEffectIntent,
    pub(crate) seal: Arc<RequestSeal>,
}
