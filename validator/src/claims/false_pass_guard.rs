use super::definition::ClaimDefinition;
use super::evidence::{Actor, ActorRole, EvidenceEnvelope};
use super::false_pass::{
    OBSERVATION_METHOD, expected_observer_id, method_sequence, validate_named_control_execution,
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
) -> Vec<String> {
    let observed = envelope
        .false_pass_execution
        .as_ref()
        .expect("caller checks execution observation");
    let execution = observed.execution();
    let observer = observed.observer();
    let mut reasons = Vec::new();
    if observed.verify_integrity().is_err() {
        reasons.push("claims-false-pass-observation-mutated".to_owned());
    }
    if validate_named_control_execution(registry_digest, definition, execution).is_err() {
        reasons.push("claims-false-pass-named-control-definition-mismatch".to_owned());
    }
    if expected_observer_id(registry_digest, definition, execution.control_id())
        .map_or(true, |expected| expected != observer.actor_id)
        || method_sequence(observed.observation_method(), OBSERVATION_METHOD)
            != method_sequence(
                execution.execution_method(),
                super::false_pass::EXECUTION_METHOD,
            )
    {
        reasons.push("claims-false-pass-observer-authority-mismatch".to_owned());
    }
    if execution.live_context_id() != context
        || execution.candidate_id() != candidate
        || execution.live_context_id() != envelope.live_context_id
        || execution.candidate_id() != envelope.candidate_id
    {
        reasons.push("claims-false-pass-context-or-candidate-mismatch".to_owned());
    }
    if execution.truth_surface() != definition.truth_surface
        || execution.truth_surface() != envelope.truth_surface
    {
        reasons.push("claims-false-pass-wrong-truth-surface".to_owned());
    }
    if execution.declared_ceiling() != definition.allowed_ceiling_on_pass
        || execution.declared_ceiling() != envelope.declared_ceiling
    {
        reasons.push("claims-false-pass-unsupported-ceiling".to_owned());
    }
    if execution.started_at_unix_ms() >= execution.ended_at_unix_ms()
        || execution.ended_at_unix_ms() > observed.observed_at_unix_ms()
        || observed.observed_at_unix_ms() > envelope.observed_at_unix_ms
        || observed.observed_at_unix_ms() > now
        || now.saturating_sub(execution.ended_at_unix_ms()) > execution.max_age_ms()
        || now.saturating_sub(observed.observed_at_unix_ms()) > execution.max_age_ms()
    {
        reasons.push("claims-false-pass-execution-time-or-freshness-invalid".to_owned());
    }
    let actors = [
        execution.executor().actor_id.as_str(),
        observer.actor_id.as_str(),
        envelope.producer.actor_id.as_str(),
        reviewer.actor_id.as_str(),
    ];
    if actors.into_iter().collect::<BTreeSet<_>>().len() != actors.len() {
        reasons.push("claims-false-pass-self-authored".to_owned());
    }
    if !execution
        .executor()
        .roles
        .contains(&ActorRole::ControlExecutor)
        || execution
            .executor()
            .roles
            .contains(&ActorRole::MaterialScorer)
        || execution
            .executor()
            .roles
            .contains(&ActorRole::IndependentReviewer)
    {
        reasons.push("claims-false-pass-executor-role-invalid".to_owned());
    }
    if !observer.roles.contains(&ActorRole::ExecutionObserver)
        || !observer.roles.contains(&ActorRole::IndependentObserver)
        || observer.roles.contains(&ActorRole::EvidenceProducer)
        || observer.roles.contains(&ActorRole::MaterialScorer)
        || observer.roles.contains(&ActorRole::IndependentReviewer)
    {
        reasons.push("claims-false-pass-observer-not-independent".to_owned());
    }
    let methods = [
        envelope.method.as_str(),
        execution.execution_method(),
        observed.observation_method(),
    ];
    if methods.into_iter().collect::<BTreeSet<_>>().len() != methods.len()
        || methods.iter().any(|method| generic_method(method))
    {
        reasons.push("claims-false-pass-executor-or-observer-method-invalid".to_owned());
    }
    if execution
        .artifact_digests()
        .iter()
        .any(|digest| accepted_artifacts.contains(digest))
    {
        reasons.push("claims-false-pass-artifact-replayed".to_owned());
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
