use super::decision::{ClaimDecision, DecisionStatus};
use super::definition::{ClaimDefinition, ClaimDefinitions};
use super::evidence::{Actor, ActorRole, EvidenceKind, Observation};
use std::collections::{BTreeMap, BTreeSet};

pub struct ClaimGuard;

impl ClaimGuard {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn evaluate(
        definitions: &ClaimDefinitions,
        prior: &BTreeMap<String, ClaimDecision>,
        accepted_candidates: &BTreeMap<String, String>,
        accepted_artifacts: &BTreeSet<String>,
        invalidated_evidence: &BTreeSet<String>,
        observations: &BTreeMap<String, Observation>,
        claim_id: &str,
        context: &str,
        candidate: &str,
        now: u64,
        reviewer: &Actor,
        evidence_ids: &[String],
    ) -> ClaimDecision {
        let Some(definition) = definitions.definition(claim_id) else {
            return rejected(
                claim_id,
                context,
                candidate,
                reviewer,
                evidence_ids,
                "claims-unknown-claim",
            );
        };
        let mut reasons = reviewer_reasons(definition, reviewer);
        reasons.extend(prerequisite_reasons(definition, prior, context, candidate));
        let mut selected = Vec::new();
        if evidence_ids.is_empty()
            || evidence_ids.iter().collect::<BTreeSet<_>>().len() != evidence_ids.len()
        {
            reasons.push("claims-evidence-set-invalid".to_owned());
        }
        for evidence_id in evidence_ids {
            match observations.get(evidence_id) {
                Some(observation) => {
                    reasons.extend(evidence_reasons(
                        definitions.registry_digest(),
                        definition,
                        observation,
                        accepted_candidates,
                        accepted_artifacts,
                        invalidated_evidence,
                        context,
                        candidate,
                        now,
                        reviewer,
                    ));
                    selected.push(observation);
                }
                None => reasons.push("claims-evidence-missing".to_owned()),
            }
        }
        reasons.extend(super::coverage::obligation_reasons(
            definitions.registry_digest(),
            definition,
            &selected,
        ));
        if reasons.is_empty() {
            ClaimDecision {
                claim_id: claim_id.to_owned(),
                live_context_id: context.to_owned(),
                candidate_id: candidate.to_owned(),
                reviewer_id: reviewer.actor_id.clone(),
                status: DecisionStatus::Passed,
                evidence_ids: evidence_ids.to_vec(),
                reasons,
            }
        } else {
            rejected(
                claim_id,
                context,
                candidate,
                reviewer,
                evidence_ids,
                &reasons.join(","),
            )
        }
    }
}

fn reviewer_reasons(definition: &ClaimDefinition, reviewer: &Actor) -> Vec<String> {
    let mut reasons = Vec::new();
    if reviewer.actor_id != definition.independent_reconciler {
        reasons.push("claims-reviewer-not-designated".to_owned());
    }
    if !reviewer.roles.contains(&ActorRole::IndependentReviewer) {
        reasons.push("claims-reviewer-not-independent".to_owned());
    }
    reasons
}

fn prerequisite_reasons(
    definition: &ClaimDefinition,
    prior: &BTreeMap<String, ClaimDecision>,
    context: &str,
    candidate: &str,
) -> Vec<String> {
    definition
        .prerequisite_claim_ids
        .iter()
        .filter_map(|claim_id| match prior.get(claim_id) {
            Some(decision)
                if decision.status == DecisionStatus::Passed
                    && decision.live_context_id == context
                    && decision.candidate_id == candidate =>
            {
                None
            }
            _ => Some(format!("claims-prerequisite-not-closed:{claim_id}")),
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn evidence_reasons(
    registry_digest: &str,
    definition: &ClaimDefinition,
    observation: &Observation,
    accepted: &BTreeMap<String, String>,
    accepted_artifacts: &BTreeSet<String>,
    invalidated_evidence: &BTreeSet<String>,
    context: &str,
    candidate: &str,
    now: u64,
    reviewer: &Actor,
) -> Vec<String> {
    let envelope = observation.envelope();
    let mut reasons = Vec::new();
    if observation.validate().is_err() {
        reasons.push("claims-observation-mutated".to_owned());
    }
    if envelope.claim_id != definition.claim_id {
        reasons.push("claims-evidence-wrong-claim".to_owned());
    }
    if envelope.live_context_id != context || envelope.candidate_id != candidate {
        reasons.push("claims-evidence-context-or-candidate-mismatch".to_owned());
    }
    if now < envelope.observed_at_unix_ms
        || now.saturating_sub(envelope.observed_at_unix_ms) > envelope.max_age_ms
    {
        reasons.push("claims-evidence-stale".to_owned());
    }
    if envelope.truth_surface != definition.truth_surface {
        reasons.push("claims-evidence-wrong-truth-surface".to_owned());
    }
    if envelope.declared_ceiling != definition.allowed_ceiling_on_pass {
        reasons.push("claims-evidence-unsupported-ceiling".to_owned());
    }
    if envelope.kind != EvidenceKind::DirectObservation {
        reasons.push("claims-evidence-prohibited-substitution".to_owned());
    }
    if !envelope
        .producer
        .roles
        .contains(&ActorRole::EvidenceProducer)
        || !envelope
            .producer
            .roles
            .contains(&ActorRole::IndependentObserver)
    {
        reasons.push("claims-evidence-producer-not-independent".to_owned());
    }
    if envelope.producer.actor_id == reviewer.actor_id
        || envelope.producer.roles.contains(&ActorRole::MaterialScorer)
    {
        reasons.push("claims-self-authored-or-material-score".to_owned());
    }
    if accepted
        .get(observation.evidence_id())
        .is_some_and(|prior| prior != candidate)
    {
        reasons.push("claims-evidence-reused-across-candidates".to_owned());
    }
    if invalidated_evidence.contains(observation.evidence_id()) {
        reasons.push("claims-evidence-invalidated-by-repair".to_owned());
    }
    if envelope
        .artifact_digests
        .iter()
        .any(|digest| accepted_artifacts.contains(digest))
    {
        reasons.push("claims-evidence-artifact-replayed".to_owned());
    }
    if envelope.false_pass_execution.is_some() {
        reasons.extend(super::false_pass_guard::reasons(
            registry_digest,
            definition,
            envelope,
            accepted_artifacts,
            context,
            candidate,
            now,
            reviewer,
        ));
    }
    reasons
}

fn rejected(
    claim_id: &str,
    context: &str,
    candidate: &str,
    reviewer: &Actor,
    evidence_ids: &[String],
    reason: &str,
) -> ClaimDecision {
    ClaimDecision {
        claim_id: claim_id.to_owned(),
        live_context_id: context.to_owned(),
        candidate_id: candidate.to_owned(),
        reviewer_id: reviewer.actor_id.clone(),
        status: DecisionStatus::Rejected,
        evidence_ids: evidence_ids.to_vec(),
        reasons: reason.split(',').map(ToOwned::to_owned).collect(),
    }
}
