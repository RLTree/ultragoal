use super::{
    ADVISORY_SELECTION_NO_CLAIM, AdvisoryLens, AdvisorySelectionDisposition,
    AdvisorySelectionRequest, EngineeringAdvisorySelection,
};
use crate::digest;
use std::collections::BTreeSet;

pub(super) struct SelectionDetails {
    pub(super) primary_lens: Option<AdvisoryLens>,
    pub(super) supporting_lenses: Vec<AdvisoryLens>,
    pub(super) activation_reasons: Vec<String>,
    pub(super) missing_inputs: BTreeSet<String>,
    pub(super) plain_language_result: &'static str,
    pub(super) plain_language_next_action: &'static str,
}

pub(super) fn selection(
    request: &AdvisorySelectionRequest,
    input_fingerprint: String,
    disposition: AdvisorySelectionDisposition,
    details: SelectionDetails,
) -> EngineeringAdvisorySelection {
    let selection_id = digest::bytes(
        &serde_json::to_vec(&(
            &input_fingerprint,
            disposition,
            details.primary_lens,
            &details.supporting_lenses,
        ))
        .expect("selection identity serializes"),
    );
    EngineeringAdvisorySelection {
        schema_version: "EngineeringAdvisorySelection-v1".to_owned(),
        selection_id,
        input_fingerprint,
        candidate_id: request.candidate_id.clone(),
        context_id: request.context_id.clone(),
        disposition,
        primary_lens: details.primary_lens,
        supporting_lenses: details.supporting_lenses,
        activation_reasons: details.activation_reasons,
        assumptions: BTreeSet::from(["advisory output is proposal-only".to_owned()]),
        missing_inputs: details.missing_inputs,
        unsupported_surfaces: BTreeSet::from([
            "effects".to_owned(),
            "claims".to_owned(),
            "readiness".to_owned(),
            "release".to_owned(),
            "completion".to_owned(),
        ]),
        adopting_owner: "OWN-ULTRA-ROOT".to_owned(),
        invalidation_conditions: BTreeSet::from([
            "candidate".to_owned(),
            "context".to_owned(),
            "lifecycle".to_owned(),
            "truth_loop".to_owned(),
            "risk_or_oracle".to_owned(),
            "failure_or_recovery".to_owned(),
            "profile_or_plugin".to_owned(),
        ]),
        plain_language_result: details.plain_language_result.to_owned(),
        plain_language_next_action: details.plain_language_next_action.to_owned(),
        claim_ceiling: "proposal_only_no_claim".to_owned(),
        no_claim_statement: ADVISORY_SELECTION_NO_CLAIM.to_owned(),
    }
}

pub(super) fn fingerprint(request: &AdvisorySelectionRequest) -> String {
    let mut current = request.clone();
    current.prior_selection = None;
    digest::bytes(&serde_json::to_vec(&current).expect("selection inputs serialize"))
}
