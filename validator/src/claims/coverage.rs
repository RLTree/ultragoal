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
) -> Vec<String> {
    let expected = definition.required_obligations();
    let mut reasons = Vec::new();
    let mut observed = BTreeSet::new();
    let mut methods = BTreeSet::new();
    let mut artifacts = BTreeSet::new();
    let mut outputs = BTreeMap::new();
    let mut execution_ids = BTreeSet::new();

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
                .any(|digest| outputs.insert(digest, obligation).is_some())
        {
            reasons.push("claims-obligation-shared-output".to_owned());
        }
        if let Some(observed_execution) = &envelope.false_pass_execution {
            let execution = observed_execution.execution();
            if !execution_ids.insert(execution.execution_id()) {
                reasons.push("claims-false-pass-execution-replayed".to_owned());
            }
            if !methods.insert(execution.execution_method())
                || !methods.insert(observed_execution.observation_method())
            {
                reasons.push("claims-false-pass-generic-method-reused".to_owned());
            }
            if execution.artifact_digests().len() != 1
                || execution
                    .artifact_digests()
                    .iter()
                    .any(|digest| !artifacts.insert(digest.as_str()))
            {
                reasons.push("claims-false-pass-shared-artifact".to_owned());
            }
            if execution.output_digests().len() != 1
                || execution
                    .output_digests()
                    .values()
                    .any(|digest| outputs.insert(digest, obligation).is_some())
            {
                reasons.push("claims-false-pass-shared-output".to_owned());
            }
        }
        reasons.extend(binding_reasons(
            registry_digest,
            definition,
            obligation,
            observation,
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
) -> Vec<String> {
    let envelope = observation.envelope();
    let mut reasons = Vec::new();
    if obligation.kind != ObligationKind::FalsePassControl
        && envelope.false_pass_execution.is_some()
    {
        reasons.push(format!(
            "claims-false-pass-execution-on-wrong-obligation:{}",
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
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !matches!(envelope.result, ObligationResult::Supported { .. }) {
        reasons.push(format!(
            "claims-false-pass-report-only-result:{}",
            obligation.id
        ));
    }
    let Some(observed) = &envelope.false_pass_execution else {
        reasons.push(format!(
            "claims-false-pass-execution-unobserved:{}",
            obligation.id
        ));
        return reasons;
    };
    let execution = observed.execution();
    let expected = expected_control_definition(registry_digest, definition, &obligation.id);
    if execution.control_id() != obligation.id
        || !expected.as_ref().is_ok_and(|item| {
            item.expected_failure_contract == execution.expected_failure_contract()
        })
    {
        reasons.push(format!(
            "claims-false-pass-control-or-contract-mismatch:{}",
            obligation.id
        ));
    }
    if envelope.result.result_digest() != observed.observed_digest() {
        reasons.push(format!(
            "claims-false-pass-result-not-execution-bound:{}",
            obligation.id
        ));
    }
    if envelope.inputs.len() != 1
        || envelope.inputs.get(&obligation.id)
            != Some(&execution.negative_stimulus_digest().to_owned())
    {
        reasons.push(format!(
            "claims-false-pass-negative-stimulus-unbound:{}",
            obligation.id
        ));
    }
    if envelope.environment_and_tools.len() != 1
        || envelope
            .environment_and_tools
            .get(execution.execution_method())
            != Some(&execution.executor_tool_digest().to_owned())
    {
        reasons.push(format!(
            "claims-false-pass-executor-method-unbound:{}",
            obligation.id
        ));
    }
    let outcome_digest = execution.actual_causal_outcome().outcome_digest();
    if execution.output_digests().len() != 1
        || execution
            .output_digests()
            .get(&obligation.id)
            .map(String::as_str)
            != Some(outcome_digest)
    {
        reasons.push(format!(
            "claims-false-pass-causal-output-unbound:{}",
            obligation.id
        ));
    }
    if !execution
        .actual_causal_outcome()
        .is_expected_failure(execution.expected_failure_contract())
    {
        reasons.push(format!(
            "claims-false-pass-control-not-executed-to-expected-failure:{}",
            obligation.id
        ));
    }
    reasons
}
