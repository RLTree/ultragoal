use super::evidence::Actor;

pub(super) struct SemanticControlModelDraft {
    pub(super) model_id: String,
    pub(super) registry_digest: String,
    pub(super) claim_id: String,
    pub(super) control_id: String,
    pub(super) control_definition_digest: String,
    pub(super) model_spec_digest: String,
    pub(super) authority_nonce: String,
    pub(super) negative_stimulus_digest: String,
    pub(super) expected_failure_contract: String,
    pub(super) modeler: Actor,
    pub(super) model_method: String,
    pub(super) model_implementation_digest: String,
    pub(super) modeled_at_unix_ms: u64,
    pub(super) live_context_id: String,
    pub(super) candidate_id: String,
    pub(super) max_age_ms: u64,
    pub(super) truth_surface: String,
    pub(super) declared_ceiling: String,
    pub(super) modeled_result_digest: String,
}
