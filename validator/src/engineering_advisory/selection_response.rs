use super::{
    ADVISORY_SELECTION_NO_CLAIM, ADVISORY_SELECTION_SCHEMA, AdvisoryLens,
    AdvisorySelectionDisposition, AdvisorySelectionRequest, EngineeringAdvisorySelection,
};
use crate::digest;
use crate::plugin_product::engineering_advisory::qualify_skill;
use std::collections::BTreeSet;

const SELECTION_IDENTITY_DOMAIN: &str = "EngineeringAdvisorySelectionIdentity-v2";

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
    let qualified_primary_skill = details
        .primary_lens
        .and_then(|lens| qualify_skill(lens.skill_name()));
    let qualified_supporting_skills = details
        .supporting_lenses
        .iter()
        .filter_map(|lens| qualify_skill(lens.skill_name()))
        .collect::<Vec<_>>();
    let agentic_candidate = request
        .profile
        .as_ref()
        .map(|profile| profile.candidate_binding.clone());
    let mut selection = EngineeringAdvisorySelection {
        schema_version: ADVISORY_SELECTION_SCHEMA.to_owned(),
        selection_id: String::new(),
        input_fingerprint,
        candidate_id: request.candidate_id.clone(),
        context_id: request.context_id.clone(),
        pack_set_digest: request.pack_set_digest.clone(),
        profile_digest: request.profile_digest.clone(),
        selector_version: request.selector_version.clone(),
        agentic_candidate,
        disposition,
        primary_lens: details.primary_lens,
        qualified_primary_skill,
        supporting_lenses: details.supporting_lenses,
        qualified_supporting_skills,
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
    };
    selection.selection_id = selection_identity(&selection);
    selection
}

pub(super) fn selection_identity(selection: &EngineeringAdvisorySelection) -> String {
    let mut canonical = selection.clone();
    canonical.selection_id.clear();
    digest::bytes(
        &serde_json::to_vec(&(SELECTION_IDENTITY_DOMAIN, canonical))
            .expect("selection identity serializes"),
    )
}

pub(super) fn fingerprint(request: &AdvisorySelectionRequest) -> String {
    let mut current = request.clone();
    current.prior_selection = None;
    digest::bytes(&serde_json::to_vec(&current).expect("selection inputs serialize"))
}
