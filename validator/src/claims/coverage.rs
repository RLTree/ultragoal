use super::definition::ClaimDefinition;
use super::evidence::{
    ClaimObligation, EvidenceEnvelope, ObligationKind, ObligationResult, Observation,
};
use super::false_pass::expected_control_definition;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn obligation_reasons(
    registry_digest: &str,
    definition: &ClaimDefinition,
    observations: &[&Observation],
    allow_semantic_models: bool,
) -> Vec<String> {
    let expected = definition.required_obligations();
    let mut reasons = Vec::new();
    let mut observed = BTreeSet::new();
    let mut methods = BTreeSet::new();
    let mut artifacts = BTreeSet::new();
    let mut outputs: BTreeMap<String, &ClaimObligation> = BTreeMap::new();
    let mut model_ids = BTreeSet::new();

    for observation in observations {
        let envelope = observation.envelope();
        let obligation = &envelope.obligation;
        if !expected.contains(obligation) {
            reasons.push(format!("claims-obligation-unknown:{}", obligation.id));
        }
        if !observed.insert(obligation.clone()) {
            reasons.push(format!("claims-obligation-duplicate:{}", obligation.id));
        }
        if !methods.insert(envelope.method.as_str()) {
            reasons.push("claims-obligation-generic-method-reused".to_owned());
        }
        if envelope.artifact_digests.len() != 1
            || envelope
                .artifact_digests
                .iter()
                .any(|digest| !artifacts.insert(digest.as_str()))
        {
            reasons.push("claims-obligation-shared-artifact".to_owned());
        }
        if envelope.outputs.len() != 1
            || envelope
                .outputs
                .values()
                .any(|digest| outputs.insert(digest.to_owned(), obligation).is_some())
        {
            reasons.push("claims-obligation-shared-output".to_owned());
        }
        if let Some(observed_model) = &envelope.false_pass_model {
            let model = observed_model.model();
            if !model_ids.insert(model.model_id()) {
                reasons.push("claims-semantic-control-model-replayed".to_owned());
            }
            if !methods.insert(model.model_method())
                || !methods.insert(observed_model.observation_method())
            {
                reasons.push("claims-semantic-control-generic-method-reused".to_owned());
            }
            if !artifacts.insert(model.model_record_digest()) {
                reasons.push("claims-semantic-control-shared-model-record".to_owned());
            }
            if outputs
                .insert(model.modeled_result_digest().to_owned(), obligation)
                .is_some()
            {
                reasons.push("claims-semantic-control-shared-modeled-result".to_owned());
            }
        }
        reasons.extend(binding_reasons(
            registry_digest,
            definition,
            obligation,
            observation,
            allow_semantic_models,
        ));
    }
    for missing in expected.difference(&observed) {
        reasons.push(format!("claims-obligation-missing:{}", missing.id));
    }
    reasons
}

fn binding_reasons(
    registry_digest: &str,
    definition: &ClaimDefinition,
    obligation: &ClaimObligation,
    observation: &Observation,
    allow_semantic_models: bool,
) -> Vec<String> {
    let envelope = observation.envelope();
    let mut reasons = Vec::new();
    if obligation.kind != ObligationKind::FalsePassControl && envelope.false_pass_model.is_some() {
        reasons.push(format!(
            "claims-false-pass-model-on-wrong-obligation:{}",
            obligation.id
        ));
    }
    if envelope.outputs.get(&obligation.id).map(String::as_str)
        != Some(envelope.result.result_digest())
    {
        reasons.push(format!(
            "claims-obligation-bare-assertion:{}",
            obligation.id
        ));
    }
    match obligation.kind {
        ObligationKind::RequiredSurface | ObligationKind::RequiredDecision
            if !envelope.inputs.contains_key(&obligation.id) =>
        {
            reasons.push(format!("claims-obligation-input-unbound:{}", obligation.id));
        }
        ObligationKind::RequiredTool
            if !envelope.environment_and_tools.contains_key(&obligation.id) =>
        {
            reasons.push(format!("claims-obligation-tool-unbound:{}", obligation.id));
        }
        ObligationKind::FalsePassControl => {
            reasons.extend(false_pass_binding_reasons(
                registry_digest,
                definition,
                obligation,
                envelope,
                allow_semantic_models,
            ));
        }
        _ => {}
    }
    if obligation.kind != ObligationKind::FalsePassControl
        && !matches!(envelope.result, ObligationResult::Supported { .. })
    {
        reasons.push(format!("claims-obligation-unsupported:{}", obligation.id));
    }
    if matches!(envelope.result, ObligationResult::Contradicted { .. }) {
        reasons.push(format!("claims-obligation-contradicted:{}", obligation.id));
    }
    reasons
}

fn false_pass_binding_reasons(
    registry_digest: &str,
    definition: &ClaimDefinition,
    obligation: &ClaimObligation,
    envelope: &EvidenceEnvelope,
    allow_semantic_models: bool,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !matches!(envelope.result, ObligationResult::Supported { .. }) {
        reasons.push(format!(
            "claims-false-pass-report-only-result:{}",
            obligation.id
        ));
    }
    let Some(observed) = &envelope.false_pass_model else {
        reasons.push(format!(
            "claims-false-pass-proof-unobserved:{}",
            obligation.id
        ));
        return reasons;
    };
    if !allow_semantic_models {
        reasons.push(format!(
            "claims-false-pass-semantic-model-not-executed-proof:{}",
            obligation.id
        ));
    }
    let model = observed.model();
    let expected = expected_control_definition(registry_digest, definition, &obligation.id);
    if model.control_id() != obligation.id
        || !expected
            .as_ref()
            .is_ok_and(|item| item.expected_failure_contract == model.expected_failure_contract())
    {
        reasons.push(format!(
            "claims-false-pass-control-or-contract-mismatch:{}",
            obligation.id
        ));
    }
    if envelope.result.result_digest() != observed.observed_digest() {
        reasons.push(format!(
            "claims-false-pass-result-not-model-bound:{}",
            obligation.id
        ));
    }
    if envelope.inputs.len() != 1
        || envelope.inputs.get(&obligation.id) != Some(&model.negative_stimulus_digest().to_owned())
    {
        reasons.push(format!(
            "claims-false-pass-negative-stimulus-unbound:{}",
            obligation.id
        ));
    }
    if envelope.environment_and_tools.len() != 1
        || envelope.environment_and_tools.get(model.model_method())
            != Some(&model.model_implementation_digest().to_owned())
    {
        reasons.push(format!(
            "claims-false-pass-model-method-unbound:{}",
            obligation.id
        ));
    }
    if model.modeled_result_digest().is_empty() {
        reasons.push(format!(
            "claims-false-pass-modeled-result-unbound:{}",
            obligation.id
        ));
    }
    reasons
}
