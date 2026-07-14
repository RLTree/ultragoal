use super::claims::{DecisionLedger, DecisionStatus, EvidenceKind, Observation};
use super::scenario::{
    candidate_id, context_id, definitions, now, observations_for, pass_before, pass_claim,
    reviewer, submit,
};

#[test]
fn stale_context_candidate_surface_ceiling_and_surrogate_evidence_reject() {
    let definitions = definitions();
    for case in [
        "stale",
        "context",
        "candidate",
        "surface",
        "ceiling",
        "receipt",
        "test",
        "telemetry",
        "manifest",
        "worker-summary",
    ] {
        let claim_id = "CL-PACKAGE";
        let mut ledger = super::scenario::semantic_model_ledger();
        pass_before(
            &mut ledger,
            &definitions,
            claim_id,
            &format!("{case}-prior"),
        );
        let mut observations = observations_for(&definitions, claim_id, case);
        let mut envelope = observations.remove(0).envelope().clone();
        match case {
            "stale" => {
                envelope.observed_at_unix_ms = 1;
                envelope.max_age_ms = 1;
            }
            "context" => envelope.live_context_id = "wrong-context".to_owned(),
            "candidate" => envelope.candidate_id = "wrong-candidate".to_owned(),
            "surface" => envelope.truth_surface = "wrong-surface".to_owned(),
            "ceiling" => envelope.declared_ceiling = "complete".to_owned(),
            "receipt" => envelope.kind = EvidenceKind::Receipt,
            "test" => envelope.kind = EvidenceKind::TestOutput,
            "telemetry" => envelope.kind = EvidenceKind::Telemetry,
            "manifest" => envelope.kind = EvidenceKind::Manifest,
            "worker-summary" => envelope.kind = EvidenceKind::WorkerSummary,
            _ => unreachable!(),
        }
        observations.push(envelope.observe().expect("mutated observation"));
        let ids = submit(&mut ledger, observations);
        let decision = ledger.decide(
            &definitions,
            claim_id,
            context_id(),
            candidate_id(),
            now(),
            &reviewer(&definitions, claim_id),
            &ids,
        );
        assert_eq!(decision.status, DecisionStatus::Rejected, "{case}");
    }
}

#[test]
fn tampered_observation_and_malformed_digest_are_rejected() {
    let definitions = definitions();
    let claim_id = "CL-SOURCE";
    let observation = observations_for(&definitions, claim_id, "tampered").remove(0);
    let mut value = serde_json::to_value(observation).expect("encode observation");
    value["observed_digest"] = serde_json::Value::String("sha256:forged".to_owned());
    let tampered: Observation = serde_json::from_value(value).expect("decode tampered");
    let mut ledger = super::scenario::semantic_model_ledger();
    let ids = submit(&mut ledger, vec![tampered]);
    let decision = ledger.decide(
        &definitions,
        claim_id,
        context_id(),
        candidate_id(),
        now(),
        &reviewer(&definitions, claim_id),
        &ids,
    );
    assert_eq!(decision.status, DecisionStatus::Rejected);
    assert!(
        decision
            .reasons
            .iter()
            .any(|reason| reason.contains("mutated"))
    );

    let mut envelope = observations_for(&definitions, claim_id, "bad-digest")
        .remove(0)
        .envelope()
        .clone();
    let key = envelope.obligation.id.clone();
    envelope.outputs.insert(key, "bare-assertion".to_owned());
    assert!(envelope.observe().is_err());
}

#[test]
fn prerequisite_bypass_unknown_claim_and_duplicate_evidence_id_reject() {
    let definitions = definitions();
    let mut ledger = super::scenario::semantic_model_ledger();
    let ids = submit(
        &mut ledger,
        observations_for(&definitions, "CL-PACKAGE", "bypass"),
    );
    let bypass = ledger.decide(
        &definitions,
        "CL-PACKAGE",
        context_id(),
        candidate_id(),
        now(),
        &reviewer(&definitions, "CL-PACKAGE"),
        &ids,
    );
    assert_eq!(bypass.status, DecisionStatus::Rejected);
    assert!(
        bypass
            .reasons
            .iter()
            .any(|reason| reason.contains("prerequisite"))
    );

    let unknown = ledger.decide(
        &definitions,
        "CL-UNKNOWN",
        context_id(),
        candidate_id(),
        now(),
        &reviewer(&definitions, "CL-PACKAGE"),
        &[],
    );
    assert_eq!(unknown.status, DecisionStatus::Rejected);

    let duplicate = observations_for(&definitions, "CL-SOURCE", "duplicate-id").remove(0);
    ledger.submit(duplicate.clone()).expect("first submit");
    assert!(ledger.submit(duplicate).is_err());
}

#[test]
fn accepted_evidence_cannot_be_reused_across_candidates() {
    let definitions = definitions();
    let mut ledger = super::scenario::semantic_model_ledger();
    let observations = observations_for(&definitions, "CL-SOURCE", "reuse");
    let ids = submit(&mut ledger, observations);
    let first = ledger.decide(
        &definitions,
        "CL-SOURCE",
        context_id(),
        candidate_id(),
        now(),
        &reviewer(&definitions, "CL-SOURCE"),
        &ids,
    );
    assert_eq!(first.status, DecisionStatus::Passed);
    let reused = ledger.decide(
        &definitions,
        "CL-SOURCE",
        context_id(),
        "candidate-b",
        now(),
        &reviewer(&definitions, "CL-SOURCE"),
        &ids,
    );
    assert_eq!(reused.status, DecisionStatus::Rejected);
    assert!(
        reused
            .reasons
            .iter()
            .any(|reason| reason.contains("reused-across-candidates"))
    );
}

#[test]
fn exact_sets_still_pass_after_negative_matrix_setup() {
    let definitions = definitions();
    let mut ledger = super::scenario::semantic_model_ledger();
    pass_claim(&mut ledger, &definitions, "CL-SOURCE", "control");
}
