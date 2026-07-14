use super::*;

impl RoutineMediatedIntent {
    pub(crate) fn new(
        request_id: String,
        protocol_id: String,
        execution_result_scope: String,
        intent: RoutineEffectIntent,
        seal: Arc<RequestSeal>,
    ) -> Self {
        Self {
            request_id,
            protocol_id,
            execution_result_scope,
            intent,
            seal,
        }
    }

    pub(crate) fn request_id(&self) -> &str {
        &self.request_id
    }

    pub(crate) fn protocol_id(&self) -> &str {
        &self.protocol_id
    }

    pub(crate) fn execution_result_scope(&self) -> &str {
        &self.execution_result_scope
    }

    pub(crate) fn intent(&self) -> &RoutineEffectIntent {
        &self.intent
    }

    pub(crate) fn require_current(&self) -> Result<(), RoutineError> {
        self.seal.require_order(self.intent.plan_order())
    }

    pub(crate) fn advance(&self) -> Result<(), RoutineError> {
        self.seal.advance(self.intent.plan_order())
    }

    pub(crate) fn same_issuance_witness(&self, witness: &RoutineMediatedWitness) -> bool {
        Arc::ptr_eq(&self.seal, &witness.seal)
            && self.request_id == witness.request_id
            && self.protocol_id == witness.protocol_id
            && self.execution_result_scope == witness.execution_result_scope
            && self.intent.intent_id == witness.intent_id
            && self.intent.plan_order == witness.plan_order
            && self.intent.node_id == witness.node_id
    }
}

/// An exact reuse expectation carrying the opaque issuance that authorized it.
#[must_use = "a mediated expectation must bind one witness or be explicitly discarded"]
pub(crate) struct RoutineMediatedExpectation {
    pub(crate) request_id: String,
    pub(crate) protocol_id: String,
    pub(crate) intent_id: String,
    pub(crate) plan_order: usize,
    pub(crate) node_id: String,
    pub(crate) execution_result_scope: String,
    pub(crate) expectation: ReuseExpectation,
    pub(crate) seal: Arc<RequestSeal>,
}

impl RoutineMediatedExpectation {
    pub(crate) fn new(token: &RoutineMediatedIntent, expectation: ReuseExpectation) -> Self {
        Self {
            request_id: token.request_id.clone(),
            protocol_id: token.protocol_id.clone(),
            intent_id: token.intent.intent_id.clone(),
            plan_order: token.intent.plan_order,
            node_id: token.intent.node_id.clone(),
            execution_result_scope: token.execution_result_scope.clone(),
            expectation,
            seal: Arc::clone(&token.seal),
        }
    }

    pub(crate) fn expectation(&self) -> &ReuseExpectation {
        &self.expectation
    }

    pub(crate) fn node_id(&self) -> &str {
        &self.node_id
    }

    pub(crate) fn execution_result_scope(&self) -> &str {
        &self.execution_result_scope
    }

    pub(crate) fn require_current(&self) -> Result<(), RoutineError> {
        self.seal.require_order(self.plan_order)
    }

    pub(crate) fn into_witness(self, disposition: ReportDisposition) -> RoutineMediatedWitness {
        RoutineMediatedWitness {
            request_id: self.request_id,
            protocol_id: self.protocol_id,
            intent_id: self.intent_id,
            plan_order: self.plan_order,
            node_id: self.node_id,
            execution_result_scope: self.execution_result_scope,
            disposition,
            seal: self.seal,
        }
    }
}

/// A complete executed or reused witness sealed to one request issuance.
#[must_use = "a mediated witness must be observed once or explicitly discarded"]
pub(crate) struct RoutineMediatedWitness {
    pub(crate) request_id: String,
    pub(crate) protocol_id: String,
    pub(crate) intent_id: String,
    pub(crate) plan_order: usize,
    pub(crate) node_id: String,
    pub(crate) execution_result_scope: String,
    pub(crate) disposition: ReportDisposition,
    pub(crate) seal: Arc<RequestSeal>,
}

pub(crate) struct RoutineMediatedOutcome {
    pub(crate) request_id: String,
    pub(crate) protocol_id: String,
    pub(crate) intent_id: String,
    pub(crate) plan_order: usize,
    pub(crate) node_id: String,
    pub(crate) observed: bool,
    pub(crate) disposition: ReportDisposition,
    pub(crate) seal: Arc<RequestSeal>,
}

impl RoutineMediatedOutcome {
    pub(crate) fn observed(token: RoutineMediatedIntent, disposition: ReportDisposition) -> Self {
        Self {
            request_id: token.request_id,
            protocol_id: token.protocol_id,
            intent_id: token.intent.intent_id,
            plan_order: token.intent.plan_order,
            node_id: token.intent.node_id,
            observed: true,
            disposition,
            seal: token.seal,
        }
    }

    #[cfg(test)]
    pub(crate) fn test_with_identity(
        mut self,
        request_id: impl Into<String>,
        protocol_id: impl Into<String>,
    ) -> Self {
        self.request_id = request_id.into();
        self.protocol_id = protocol_id.into();
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_intent(
        mut self,
        intent_id: impl Into<String>,
        plan_order: usize,
        node_id: impl Into<String>,
    ) -> Self {
        self.intent_id = intent_id.into();
        self.plan_order = plan_order;
        self.node_id = node_id.into();
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_observed(mut self, observed: bool) -> Self {
        self.observed = observed;
        self
    }
}

pub(crate) fn mediation_rows(intents: &[RoutineEffectIntent]) -> Vec<MediatedExpectedRow> {
    intents
        .iter()
        .map(|intent| MediatedExpectedRow {
            intent_id: intent.intent_id.clone(),
            plan_order: intent.plan_order,
            node_id: intent.node_id.clone(),
        })
        .collect()
}
