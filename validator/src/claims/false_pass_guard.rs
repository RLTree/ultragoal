use super::definition::ClaimDefinition;
use super::evidence::{Actor, ActorRole, EvidenceEnvelope};
use super::false_pass::{
    MODEL_METHOD, MODEL_OBSERVATION_METHOD, expected_model_observer_id, method_sequence,
    validate_named_control_model,
};
use std::collections::BTreeSet;

#[allow(clippy::too_many_arguments)]
pub(super) fn reasons(
    registry_digest: &str,
    definition: &ClaimDefinition,
    envelope: &EvidenceEnvelope,
    accepted_artifacts: &BTreeSet<String>,
    context: &str,
    candidate: &str,
    now: u64,
    reviewer: &Actor,
    allow_semantic_models: bool,
) -> Vec<String> {
    let observed = envelope
        .false_pass_model
        .as_ref()
        .expect("caller checks semantic control model");
    let model = observed.model();
    let observer = observed.observer();
    let mut reasons = Vec::new();
    if !allow_semantic_models {
        reasons.push("claims-false-pass-semantic-model-not-executed-proof".to_owned());
    }
    if observed.verify_integrity().is_err() {
        reasons.push("claims-semantic-control-observation-mutated".to_owned());
    }
    if validate_named_control_model(registry_digest, definition, model).is_err() {
        reasons.push("claims-semantic-control-model-definition-mismatch".to_owned());
    }
    if expected_model_observer_id(registry_digest, definition, model.control_id())
        .map_or(true, |expected| expected != observer.actor_id)
        || method_sequence(observed.observation_method(), MODEL_OBSERVATION_METHOD)
            != method_sequence(model.model_method(), MODEL_METHOD)
    {
        reasons.push("claims-semantic-control-observer-authority-mismatch".to_owned());
    }
    if model.live_context_id() != context
        || model.candidate_id() != candidate
        || model.live_context_id() != envelope.live_context_id
        || model.candidate_id() != envelope.candidate_id
    {
        reasons.push("claims-semantic-control-context-or-candidate-mismatch".to_owned());
    }
    if model.truth_surface() != definition.truth_surface
        || model.truth_surface() != envelope.truth_surface
    {
        reasons.push("claims-semantic-control-wrong-truth-surface".to_owned());
    }
    if model.declared_ceiling() != definition.allowed_ceiling_on_pass
        || model.declared_ceiling() != envelope.declared_ceiling
    {
        reasons.push("claims-semantic-control-unsupported-ceiling".to_owned());
    }
    if model.modeled_at_unix_ms() > observed.observed_at_unix_ms()
        || observed.observed_at_unix_ms() > envelope.observed_at_unix_ms
        || observed.observed_at_unix_ms() > now
        || now.saturating_sub(model.modeled_at_unix_ms()) > model.max_age_ms()
        || now.saturating_sub(observed.observed_at_unix_ms()) > model.max_age_ms()
    {
        reasons.push("claims-semantic-control-time-or-freshness-invalid".to_owned());
    }
    let actors = [
        model.modeler().actor_id.as_str(),
        observer.actor_id.as_str(),
        envelope.producer.actor_id.as_str(),
        reviewer.actor_id.as_str(),
    ];
    if actors.into_iter().collect::<BTreeSet<_>>().len() != actors.len() {
        reasons.push("claims-semantic-control-self-authored".to_owned());
    }
    if model.modeler().roles != BTreeSet::from([ActorRole::SemanticModeler]) {
        reasons.push("claims-semantic-control-modeler-role-invalid".to_owned());
    }
    if observer.roles
        != BTreeSet::from([
            ActorRole::IndependentObserver,
            ActorRole::SemanticModelObserver,
        ])
    {
        reasons.push("claims-semantic-control-observer-role-invalid".to_owned());
    }
    let methods = [
        envelope.method.as_str(),
        model.model_method(),
        observed.observation_method(),
    ];
    if methods.into_iter().collect::<BTreeSet<_>>().len() != methods.len()
        || methods.iter().any(|method| generic_method(method))
    {
        reasons.push("claims-semantic-control-method-invalid".to_owned());
    }
    if accepted_artifacts.contains(model.model_record_digest()) {
        reasons.push("claims-semantic-control-model-replayed".to_owned());
    }
    reasons
}

fn generic_method(method: &str) -> bool {
    let normalized = method.to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "generic" | "generic-method" | "probe" | "probe-tool" | "report" | "report-only" | "tool"
    )
}
