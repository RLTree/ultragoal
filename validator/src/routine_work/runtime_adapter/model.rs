use serde::Serialize;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use crate::routine_work::{
    RepoPath, ReportDisposition, ReportStatus, ReuseExpectation, RoutineBinding, RoutineError,
    RoutineErrorId,
};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RoutineInvocationSpec {
    pub(super) node_id: String,
    pub(super) tool_name: String,
    pub(super) tool_identity_sha256: String,
    pub(super) program_path_hex: String,
    pub(super) program_sha256: String,
    pub(super) program_byte_length: u64,
    pub(super) program_unix_mode: Option<u32>,
    pub(super) arguments: Vec<String>,
    pub(super) timeout_ms: u64,
    pub(super) output_budget_bytes: u64,
    pub(super) declared_output_scopes: Vec<RepoPath>,
}

impl RoutineInvocationSpec {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn bound(
        node_id: String,
        tool_name: String,
        tool_identity_sha256: String,
        program_path_hex: String,
        program_sha256: String,
        program_byte_length: u64,
        program_unix_mode: Option<u32>,
        arguments: Vec<String>,
        timeout_ms: u64,
        output_budget_bytes: u64,
        declared_output_scopes: Vec<RepoPath>,
    ) -> Self {
        Self {
            node_id,
            tool_name,
            tool_identity_sha256,
            program_path_hex,
            program_sha256,
            program_byte_length,
            program_unix_mode,
            arguments,
            timeout_ms,
            output_budget_bytes,
            declared_output_scopes,
        }
    }

    pub(crate) fn node_id(&self) -> &str {
        &self.node_id
    }

    #[cfg(test)]
    pub(crate) fn test_with_node_id(mut self, node_id: impl Into<String>) -> Self {
        self.node_id = node_id.into();
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_tool_identity(mut self, identity: impl Into<String>) -> Self {
        self.tool_identity_sha256 = identity.into();
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_program_sha256(mut self, identity: impl Into<String>) -> Self {
        self.program_sha256 = identity.into();
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_arguments(mut self, arguments: Vec<String>) -> Self {
        self.arguments = arguments;
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_output_budget_bytes(mut self, output_budget_bytes: u64) -> Self {
        self.output_budget_bytes = output_budget_bytes;
        self
    }

    #[cfg(test)]
    pub(crate) fn test_with_output_scopes(mut self, scopes: Vec<RepoPath>) -> Self {
        self.declared_output_scopes = scopes;
        self
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct RoutineAdapterSpec {
    pub(super) result_scope: String,
    pub(super) invocations: Vec<RoutineInvocationSpec>,
}

impl RoutineAdapterSpec {
    pub(crate) fn new(
        result_scope: impl Into<String>,
        invocations: Vec<RoutineInvocationSpec>,
    ) -> Self {
        Self {
            result_scope: result_scope.into(),
            invocations,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RoutineEffectIntent {
    protocol_id: String,
    intent_id: String,
    plan_order: usize,
    node_id: String,
    selected_tool: String,
    tool_identity_sha256: String,
    program_path_hex: String,
    program_sha256: String,
    program_byte_length: u64,
    program_unix_mode: Option<u32>,
    argv: Vec<String>,
    working_directory: String,
    environment_policy: &'static str,
    mediation_preflight: &'static str,
    timeout_ms: u64,
    output_budget_bytes: u64,
    declared_output_scopes: Vec<RepoPath>,
    expected_dependency_nodes: Vec<String>,
    input_id: String,
}

impl RoutineEffectIntent {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        protocol_id: String,
        intent_id: String,
        plan_order: usize,
        node_id: String,
        selected_tool: String,
        tool_identity_sha256: String,
        program_path_hex: String,
        program_sha256: String,
        program_byte_length: u64,
        program_unix_mode: Option<u32>,
        argv: Vec<String>,
        working_directory: String,
        timeout_ms: u64,
        output_budget_bytes: u64,
        declared_output_scopes: Vec<RepoPath>,
        expected_dependency_nodes: Vec<String>,
        input_id: String,
    ) -> Self {
        Self {
            protocol_id,
            intent_id,
            plan_order,
            node_id,
            selected_tool,
            tool_identity_sha256,
            program_path_hex,
            program_sha256,
            program_byte_length,
            program_unix_mode,
            argv,
            working_directory,
            environment_policy: "clear-all-no-inheritance-v1",
            mediation_preflight: "revalidate-context-candidate-tool-executable-output-scopes-before-effect-v1",
            timeout_ms,
            output_budget_bytes,
            declared_output_scopes,
            expected_dependency_nodes,
            input_id,
        }
    }

    pub(crate) fn protocol_id(&self) -> &str {
        &self.protocol_id
    }
    pub(crate) fn intent_id(&self) -> &str {
        &self.intent_id
    }
    pub(crate) fn plan_order(&self) -> usize {
        self.plan_order
    }
    pub(crate) fn node_id(&self) -> &str {
        &self.node_id
    }
    pub(crate) fn selected_tool(&self) -> &str {
        &self.selected_tool
    }
    pub(crate) fn tool_identity_sha256(&self) -> &str {
        &self.tool_identity_sha256
    }
    pub(crate) fn program_path_hex(&self) -> &str {
        &self.program_path_hex
    }
    pub(crate) fn program_sha256(&self) -> &str {
        &self.program_sha256
    }
    pub(crate) fn program_byte_length(&self) -> u64 {
        self.program_byte_length
    }
    pub(crate) fn program_unix_mode(&self) -> Option<u32> {
        self.program_unix_mode
    }
    pub(crate) fn argv(&self) -> &[String] {
        &self.argv
    }
    pub(crate) fn working_directory(&self) -> &str {
        &self.working_directory
    }
    pub(crate) fn environment_policy(&self) -> &'static str {
        self.environment_policy
    }
    pub(crate) fn mediation_preflight(&self) -> &'static str {
        self.mediation_preflight
    }
    pub(crate) fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }
    pub(crate) fn output_budget_bytes(&self) -> u64 {
        self.output_budget_bytes
    }
    pub(crate) fn declared_output_scopes(&self) -> &[RepoPath] {
        &self.declared_output_scopes
    }
    pub(crate) fn expected_dependency_nodes(&self) -> &[String] {
        &self.expected_dependency_nodes
    }
    pub(crate) fn input_id(&self) -> &str {
        &self.input_id
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RoutineNoOpProjection {
    projection_id: String,
    binding_id: String,
    context_id: String,
    candidate_id: String,
    graph_id: String,
    snapshot_id: String,
    plan_id: String,
    result_scope: String,
    selected: Vec<String>,
    status: ReportStatus,
    effect_intent_count: usize,
    support_limit: &'static str,
}

impl RoutineNoOpProjection {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        projection_id: String,
        binding: &RoutineBinding,
        graph_id: String,
        snapshot_id: String,
        plan_id: String,
        result_scope: String,
    ) -> Self {
        Self {
            projection_id,
            binding_id: binding.binding_id().to_owned(),
            context_id: binding.context_id().to_owned(),
            candidate_id: binding.candidate_id().to_owned(),
            graph_id,
            snapshot_id,
            plan_id,
            result_scope,
            selected: Vec::new(),
            status: ReportStatus::CompleteExecution,
            effect_intent_count: 0,
            support_limit: "non-effectful protocol projection only; no public command or claim",
        }
    }

    pub(crate) fn projection_id(&self) -> &str {
        &self.projection_id
    }
    pub(crate) fn context_id(&self) -> &str {
        &self.context_id
    }
    pub(crate) fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub(crate) fn plan_id(&self) -> &str {
        &self.plan_id
    }
    pub(crate) fn result_scope(&self) -> &str {
        &self.result_scope
    }
    pub(crate) fn selected(&self) -> &[String] {
        &self.selected
    }
    pub(crate) fn status(&self) -> ReportStatus {
        self.status
    }
    pub(crate) fn effect_intent_count(&self) -> usize {
        self.effect_intent_count
    }
    pub(crate) fn support_limit(&self) -> &'static str {
        self.support_limit
    }
}

pub(crate) enum PreparedRoutineExecution {
    NoOp(RoutineNoOpProjection),
    Effect(RoutineEffectRequest),
}

const REQUEST_STAGE_PREPARED: u8 = 0;
const REQUEST_STAGE_MEDIATING: u8 = 1;
const REQUEST_STAGE_RECONCILED: u8 = 2;

pub(super) struct RequestSeal {
    issuance: u64,
    seal_id: String,
    stage: AtomicU8,
    next_order: AtomicUsize,
}

impl RequestSeal {
    pub(super) fn new(issuance: u64, seal_id: String) -> Self {
        Self {
            issuance,
            seal_id,
            stage: AtomicU8::new(REQUEST_STAGE_PREPARED),
            next_order: AtomicUsize::new(0),
        }
    }

    pub(super) fn issuance(&self) -> u64 {
        self.issuance
    }

    pub(super) fn matches(&self, expected: &str) -> bool {
        self.seal_id == expected
    }

    pub(super) fn begin_mediation(&self) -> Result<(), RoutineError> {
        self.stage
            .compare_exchange(
                REQUEST_STAGE_PREPARED,
                REQUEST_STAGE_MEDIATING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| request_error("adapter-effect-request-replayed"))
    }

    pub(super) fn require_order(&self, order: usize) -> Result<(), RoutineError> {
        if self.stage.load(Ordering::Acquire) != REQUEST_STAGE_MEDIATING {
            return Err(request_error("adapter-effect-request-not-mediating"));
        }
        if self.next_order.load(Ordering::Acquire) != order {
            return Err(request_error("adapter-intent-transition-order-invalid"));
        }
        Ok(())
    }

    pub(super) fn advance(&self, order: usize) -> Result<(), RoutineError> {
        self.require_order(order)?;
        self.next_order
            .compare_exchange(order, order + 1, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| request_error("adapter-intent-transition-replayed"))
    }

    pub(super) fn require_complete(&self, expected: usize) -> Result<(), RoutineError> {
        if self.stage.load(Ordering::Acquire) != REQUEST_STAGE_MEDIATING
            || self.next_order.load(Ordering::Acquire) != expected
        {
            return Err(request_error("adapter-mediation-transition-incomplete"));
        }
        Ok(())
    }

    pub(super) fn finish(&self, expected: usize) -> Result<(), RoutineError> {
        self.require_complete(expected)?;
        self.stage
            .compare_exchange(
                REQUEST_STAGE_MEDIATING,
                REQUEST_STAGE_RECONCILED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| request_error("adapter-mediation-transition-replayed"))
    }
}

/// Opaque one-use request. It intentionally implements neither Clone, Copy,
/// Serialize, nor Deserialize and can only be consumed by reconciliation.
#[must_use = "an effect request must be mediated once or explicitly discarded"]
pub(crate) struct RoutineEffectRequest {
    pub(super) request_id: String,
    pub(super) protocol_id: String,
    pub(super) binding: RoutineBinding,
    pub(super) graph_id: String,
    pub(super) snapshot_id: String,
    pub(super) plan_id: String,
    pub(super) result_scope: String,
    pub(super) intents: Vec<RoutineEffectIntent>,
    pub(super) seal: Arc<RequestSeal>,
}

impl RoutineEffectRequest {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
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

    pub(super) fn seal_issuance(&self) -> u64 {
        self.seal.issuance()
    }

    pub(super) fn seal_matches(&self, expected: &str) -> bool {
        self.seal.matches(expected)
    }

    pub(super) fn begin_mediation(&self) -> Result<(), RoutineError> {
        self.seal.begin_mediation()
    }

    #[cfg(test)]
    pub(crate) fn test_mark_transitioned(&self) -> Result<(), RoutineError> {
        self.seal.begin_mediation()
    }
}

#[must_use = "a mediation batch must be split and reconciled once"]
pub(crate) struct RoutineMediationBatch {
    authority: RoutineMediationAuthority,
    intents: Vec<RoutineMediatedIntent>,
}

impl RoutineMediationBatch {
    pub(super) fn new(
        authority: RoutineMediationAuthority,
        intents: Vec<RoutineMediatedIntent>,
    ) -> Self {
        Self { authority, intents }
    }

    pub(crate) fn into_parts(self) -> (RoutineMediationAuthority, Vec<RoutineMediatedIntent>) {
        (self.authority, self.intents)
    }
}

pub(super) struct MediatedExpectedRow {
    pub(super) intent_id: String,
    pub(super) plan_order: usize,
    pub(super) node_id: String,
}

/// Opaque one-use authority for exactly one request issuance.
#[must_use = "mediation authority must be reconciled once or explicitly discarded"]
pub(crate) struct RoutineMediationAuthority {
    pub(super) request_id: String,
    pub(super) protocol_id: String,
    pub(super) binding: RoutineBinding,
    pub(super) graph_id: String,
    pub(super) snapshot_id: String,
    pub(super) plan_id: String,
    pub(super) requested_result_scope: String,
    pub(super) execution_result_scope: String,
    pub(super) expected: Vec<MediatedExpectedRow>,
    seal: Arc<RequestSeal>,
}

impl RoutineMediationAuthority {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
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

    pub(super) fn same_issuance(&self, outcome: &RoutineMediatedOutcome) -> bool {
        Arc::ptr_eq(&self.seal, &outcome.seal)
    }

    pub(super) fn require_complete(&self) -> Result<(), RoutineError> {
        self.seal.require_complete(self.expected.len())
    }

    pub(super) fn finish(&self) -> Result<(), RoutineError> {
        self.seal.finish(self.expected.len())
    }
}

/// Opaque one-use intent token issued by one mediation transition.
#[must_use = "a mediated intent must produce one observed outcome or be discarded"]
pub(crate) struct RoutineMediatedIntent {
    pub(super) request_id: String,
    pub(super) protocol_id: String,
    pub(super) execution_result_scope: String,
    pub(super) intent: RoutineEffectIntent,
    seal: Arc<RequestSeal>,
}

impl RoutineMediatedIntent {
    pub(super) fn new(
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

    pub(super) fn require_current(&self) -> Result<(), RoutineError> {
        self.seal.require_order(self.intent.plan_order())
    }

    pub(super) fn advance(&self) -> Result<(), RoutineError> {
        self.seal.advance(self.intent.plan_order())
    }

    pub(super) fn same_issuance_witness(&self, witness: &RoutineMediatedWitness) -> bool {
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
    request_id: String,
    protocol_id: String,
    intent_id: String,
    plan_order: usize,
    node_id: String,
    execution_result_scope: String,
    expectation: ReuseExpectation,
    seal: Arc<RequestSeal>,
}

impl RoutineMediatedExpectation {
    pub(super) fn new(token: &RoutineMediatedIntent, expectation: ReuseExpectation) -> Self {
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

    pub(super) fn node_id(&self) -> &str {
        &self.node_id
    }

    pub(super) fn execution_result_scope(&self) -> &str {
        &self.execution_result_scope
    }

    pub(super) fn require_current(&self) -> Result<(), RoutineError> {
        self.seal.require_order(self.plan_order)
    }

    pub(super) fn into_witness(self, disposition: ReportDisposition) -> RoutineMediatedWitness {
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
    request_id: String,
    protocol_id: String,
    intent_id: String,
    plan_order: usize,
    node_id: String,
    execution_result_scope: String,
    pub(super) disposition: ReportDisposition,
    seal: Arc<RequestSeal>,
}

pub(crate) struct RoutineMediatedOutcome {
    pub(super) request_id: String,
    pub(super) protocol_id: String,
    pub(super) intent_id: String,
    pub(super) plan_order: usize,
    pub(super) node_id: String,
    pub(super) observed: bool,
    pub(super) disposition: ReportDisposition,
    seal: Arc<RequestSeal>,
}

impl RoutineMediatedOutcome {
    pub(super) fn observed(token: RoutineMediatedIntent, disposition: ReportDisposition) -> Self {
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

pub(super) fn mediation_rows(intents: &[RoutineEffectIntent]) -> Vec<MediatedExpectedRow> {
    intents
        .iter()
        .map(|intent| MediatedExpectedRow {
            intent_id: intent.intent_id.clone(),
            plan_order: intent.plan_order,
            node_id: intent.node_id.clone(),
        })
        .collect()
}

pub(super) fn mediation_tokens(
    request_id: &str,
    protocol_id: &str,
    execution_result_scope: &str,
    intents: Vec<RoutineEffectIntent>,
    seal: &Arc<RequestSeal>,
) -> Vec<RoutineMediatedIntent> {
    intents
        .into_iter()
        .map(|intent| {
            RoutineMediatedIntent::new(
                request_id.to_owned(),
                protocol_id.to_owned(),
                execution_result_scope.to_owned(),
                intent,
                Arc::clone(seal),
            )
        })
        .collect()
}

pub(super) fn authority_seal(request: &RoutineEffectRequest) -> Arc<RequestSeal> {
    Arc::clone(&request.seal)
}

fn request_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}

pub(super) fn duplicate_node(values: &[RoutineInvocationSpec]) -> Option<&str> {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .find_map(|value| (!seen.insert(value.node_id.as_str())).then_some(value.node_id.as_str()))
}
