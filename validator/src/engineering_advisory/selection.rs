use super::selection_response::{SelectionDetails, fingerprint, selection};
use super::selection_validation::{
    available_skill_names, binding_is_stale, identity_is_well_formed, missing_inputs,
};
use super::{
    AdvisoryLens, AdvisoryOutcomeClass, AdvisorySelectionDisposition, AdvisorySelectionRequest,
    EngineeringAdvisorySelection,
};
use std::collections::BTreeSet;

pub const ADVISORY_SELECTION_NO_CLAIM: &str = "This advisory selection is proposal-only and cannot issue effects, accept work, or promote claims.";

pub fn select_advisory(request: &AdvisorySelectionRequest) -> EngineeringAdvisorySelection {
    let input_fingerprint = fingerprint(request);
    let missing_inputs = missing_inputs(request);
    if !missing_inputs.is_empty() {
        return selection(
            request,
            input_fingerprint,
            AdvisorySelectionDisposition::MaterialInputMissing,
            SelectionDetails {
                primary_lens: None,
                supporting_lenses: Vec::new(),
                activation_reasons: vec!["material advisory inputs are missing".to_owned()],
                missing_inputs,
                plain_language_result: "UltraGoal needs one material advisory input before it can select a lens.",
                plain_language_next_action: "Provide the named input; no advisory or effect was started.",
            },
        );
    }
    if !identity_is_well_formed(request) {
        return selection(
            request,
            input_fingerprint,
            AdvisorySelectionDisposition::StaleOrCrossCandidate,
            SelectionDetails {
                primary_lens: None,
                supporting_lenses: Vec::new(),
                activation_reasons: vec![
                    "candidate-bound advisory identity is malformed".to_owned(),
                ],
                missing_inputs: BTreeSet::new(),
                plain_language_result: "UltraGoal rejected malformed advisory identity rather than reusing stale advice.",
                plain_language_next_action: "Recompute current candidate, context, state, configuration, and profile identities.",
            },
        );
    }
    if binding_is_stale(request) {
        return selection(
            request,
            input_fingerprint,
            AdvisorySelectionDisposition::StaleOrCrossCandidate,
            SelectionDetails {
                primary_lens: None,
                supporting_lenses: Vec::new(),
                activation_reasons: vec![
                    "profile or catalog binding is stale or cross-candidate".to_owned(),
                ],
                missing_inputs: BTreeSet::new(),
                plain_language_result: "UltraGoal rejected stale advisory binding rather than reusing it.",
                plain_language_next_action: "Recompute the advisory profile and catalog from current authority.",
            },
        );
    }
    let required = required_lenses(request);
    if required.is_empty() {
        return selection(
            request,
            input_fingerprint,
            AdvisorySelectionDisposition::NoAdvisoryNeeded,
            SelectionDetails {
                primary_lens: None,
                supporting_lenses: Vec::new(),
                activation_reasons: vec![
                    "routine, already-specified, or no-change work has no material trigger"
                        .to_owned(),
                ],
                missing_inputs: BTreeSet::new(),
                plain_language_result: "No additional advisory is needed for this bounded work.",
                plain_language_next_action: "Continue through the existing UltraGoal route.",
            },
        );
    }
    if request
        .prior_selection
        .as_ref()
        .is_some_and(|prior| prior.input_fingerprint == input_fingerprint)
    {
        return selection(
            request,
            input_fingerprint,
            AdvisorySelectionDisposition::NoAdvisoryNeeded,
            SelectionDetails {
                primary_lens: None,
                supporting_lenses: Vec::new(),
                activation_reasons: vec![
                    "advice_current: no binding, evidence, or mechanism changed".to_owned(),
                ],
                missing_inputs: BTreeSet::new(),
                plain_language_result: "Existing advisory remains current; a duplicate pass would add no evidence.",
                plain_language_next_action: "Continue with the already selected UltraGoal action.",
            },
        );
    }
    let Some(available) = available_skill_names(request) else {
        return selection(
            request,
            input_fingerprint,
            AdvisorySelectionDisposition::RequiredProfileUnavailable,
            SelectionDetails {
                primary_lens: None,
                supporting_lenses: Vec::new(),
                activation_reasons: vec![
                    "the required candidate-bound Agentic profile is unavailable".to_owned(),
                ],
                missing_inputs: BTreeSet::new(),
                plain_language_result: "The required advisory profile is unavailable, so UltraGoal will not substitute advice.",
                plain_language_next_action: "Repair the named profile/catalog binding or continue only through an independent legal route.",
            },
        );
    };
    let primary = required[0];
    if let Some(unavailable) = required
        .iter()
        .find(|lens| !available.contains(lens.skill_name()))
    {
        return selection(
            request,
            input_fingerprint,
            AdvisorySelectionDisposition::RequiredProfileUnavailable,
            SelectionDetails {
                primary_lens: None,
                supporting_lenses: Vec::new(),
                activation_reasons: vec![format!(
                    "required lens {} is not enabled in the active profile",
                    unavailable.skill_name()
                )],
                missing_inputs: BTreeSet::new(),
                plain_language_result: "The active profile does not expose the required advisory lens.",
                plain_language_next_action: "Select a validated stage profile; do not infer advice from another profile.",
            },
        );
    }
    let supporting = required
        .iter()
        .copied()
        .skip(1)
        .find(|lens| lens.owner_family() != primary.owner_family())
        .filter(|lens| available.contains(lens.skill_name()))
        .into_iter()
        .collect::<Vec<_>>();
    selection(
        request,
        input_fingerprint,
        AdvisorySelectionDisposition::AdvisorySelected,
        SelectionDetails {
            primary_lens: Some(primary),
            supporting_lenses: supporting,
            activation_reasons: vec![format!(
                "{} is the smallest sufficient lens for current typed signals",
                primary.skill_name()
            )],
            missing_inputs: BTreeSet::new(),
            plain_language_result: "UltraGoal selected a proposal-only advisory lens for the current material decision.",
            plain_language_next_action: "Review the advisory result, then continue through the existing UltraGoal owner.",
        },
    )
}

fn required_lenses(request: &AdvisorySelectionRequest) -> Vec<AdvisoryLens> {
    let mut lenses = request.explicit_lens_requests.clone();
    lenses.extend(request.activation_signals.iter().copied());
    if !request.recovery_ambiguities.is_empty() {
        lenses.insert(AdvisoryLens::RustAgentDurability);
    }
    if !request.protected_invariant_ids.is_empty() || !request.contemplated_effects.is_empty() {
        lenses.insert(AdvisoryLens::AgentSecurityGovernance);
    }
    if !request.false_pass_risks.is_empty() || request.verification_oracle.is_some() {
        lenses.insert(AdvisoryLens::VerificationStrategyEngineering);
    }
    if !request.failure_classes.is_empty() {
        lenses.insert(AdvisoryLens::LoopEngineering);
    }
    if !request.product_fitness_gaps.is_empty() {
        lenses.insert(AdvisoryLens::ProductFitnessEngineering);
    }
    if !request.material_uncertainties.is_empty() || !request.authority_gaps.is_empty() {
        lenses.insert(AdvisoryLens::CodexTaskContract);
    }
    if lenses.is_empty()
        && matches!(
            request.user_outcome_class,
            AdvisoryOutcomeClass::MaterialDecision
        )
    {
        lenses.insert(AdvisoryLens::CodexTaskContract);
    }
    let mut lenses = lenses.into_iter().collect::<Vec<_>>();
    lenses.sort_by_key(|lens| lens.rank());
    lenses
}
