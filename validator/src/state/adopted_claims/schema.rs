use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(in crate::state) struct AdoptedClaimRegistry {
    pub(in crate::state) schema_version: String,
    pub(in crate::state) contract_id: String,
    pub(in crate::state) claim_count: usize,
    pub(in crate::state) claim_topological_order: Vec<String>,
    pub(in crate::state) claims: Vec<AdoptedClaimDefinition>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(in crate::state) struct AdoptedClaimDefinition {
    pub(in crate::state) claim_id: String,
    pub(in crate::state) name: String,
    pub(in crate::state) expected_behavior: String,
    pub(in crate::state) truth_surface: String,
    pub(in crate::state) prerequisite_claim_ids: Vec<String>,
    pub(in crate::state) requirement_ids: Vec<String>,
    pub(in crate::state) required_surface_ids: Vec<String>,
    pub(in crate::state) required_tool_ids: Vec<String>,
    pub(in crate::state) required_decision_ids: Vec<String>,
    pub(in crate::state) required_evidence: Vec<String>,
    pub(in crate::state) current_live_evidence: Vec<String>,
    pub(in crate::state) current_live_evidence_status: String,
    pub(in crate::state) independent_reconciler: String,
    pub(in crate::state) false_pass_controls: Vec<String>,
    pub(in crate::state) claim_guard: String,
    pub(in crate::state) repair: String,
    pub(in crate::state) rerun: String,
    pub(in crate::state) allowed_ceiling_on_pass: String,
    pub(in crate::state) initial_claim_state: String,
    pub(in crate::state) claim_decision_owner: String,
}
