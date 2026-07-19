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
    #[serde(rename = "current_live_evidence_status")]
    pub(in crate::state) live_evidence_verification: String,
    pub(in crate::state) independent_reconciler: String,
    pub(in crate::state) false_pass_controls: Vec<String>,
    pub(in crate::state) claim_guard: String,
    pub(in crate::state) repair: String,
    pub(in crate::state) rerun: String,
    pub(in crate::state) allowed_ceiling_on_pass: String,
    pub(in crate::state) initial_claim_state: String,
    pub(in crate::state) claim_decision_owner: String,
}

#[cfg(test)]
mod tests {
    use super::AdoptedClaimDefinition;

    #[test]
    fn legacy_live_evidence_key_deserializes_without_an_internal_alias() {
        let registry: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../../docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CLAIM_REGISTRY.json"
        ))
        .expect("claim registry");
        let claim: AdoptedClaimDefinition =
            serde_json::from_value(registry["claims"][0].clone()).expect("legacy claim key");
        let serialized = serde_json::to_value(claim).expect("serialize claim");
        let legacy_key = concat!("current_live_evidence_", "status");
        assert_eq!(serialized[legacy_key], "not_verified");
        assert!(serialized.get("live_evidence_verification").is_none());
    }
}
