use super::*;

pub(crate) const MATRIX_CLAIMS: [&str; 5] = [
    "CL-PACKAGE",
    "CL-INSTALL",
    "CL-ORCHESTRATION",
    "CL-RELEASE",
    "CL-COMPLETION",
];

pub(crate) fn decide(
    ledger: &mut DecisionLedger,
    definitions: &ClaimDefinitions,
    claim_id: &str,
    observations: Vec<Observation>,
) -> super::super::claims::ClaimDecision {
    let ids = submit(ledger, observations);
    ledger.decide(
        definitions,
        claim_id,
        context_id(),
        candidate_id(),
        now(),
        &reviewer(definitions, claim_id),
        &ids,
    )
}

#[test]
pub(crate) fn each_required_item_is_individually_mandatory_for_material_claims() {
    let definitions = definitions();
    for claim_id in MATRIX_CLAIMS {
        let count = definitions
            .definition(claim_id)
            .expect("definition")
            .required_obligations()
            .len();
        for omitted in 0..count {
            let mut ledger = super::super::scenario::semantic_model_ledger();
            pass_before(&mut ledger, &definitions, claim_id, "prior");
            let mut observations = observations_for(&definitions, claim_id, "omission");
            let missing = observations
                .remove(omitted)
                .envelope()
                .obligation
                .id
                .clone();
            let decision = decide(&mut ledger, &definitions, claim_id, observations);
            assert_eq!(
                decision.status,
                DecisionStatus::Rejected,
                "{claim_id}:{missing}"
            );
            assert!(
                decision
                    .reasons
                    .iter()
                    .any(|reason| reason == &format!("claims-obligation-missing:{missing}")),
                "{claim_id}:{missing}:{:?}",
                decision.reasons
            );
        }
    }
}

#[test]
pub(crate) fn one_generic_observation_never_substitutes_for_an_exact_set() {
    let definitions = definitions();
    for claim_id in MATRIX_CLAIMS {
        let mut ledger = super::super::scenario::semantic_model_ledger();
        pass_before(&mut ledger, &definitions, claim_id, "generic-prior");
        let observation = observations_for(&definitions, claim_id, "generic").remove(0);
        let decision = decide(&mut ledger, &definitions, claim_id, vec![observation]);
        assert_eq!(decision.status, DecisionStatus::Rejected, "{claim_id}");
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| reason.contains("missing"))
        );
    }
}

#[test]
pub(crate) fn bare_duplicate_unknown_and_shared_proof_reject_before_elevation() {
    let definitions = definitions();
    let claim_id = "CL-RELEASE";
    for case in ["bare", "duplicate", "unknown", "shared"] {
        let mut ledger = super::super::scenario::semantic_model_ledger();
        pass_before(
            &mut ledger,
            &definitions,
            claim_id,
            &format!("{case}-prior"),
        );
        let mut observations = observations_for(&definitions, claim_id, case);
        match case {
            "bare" => {
                let mut envelope = observations.remove(0).envelope().clone();
                let result = envelope.result.result_digest().to_owned();
                envelope.outputs.clear();
                envelope.outputs.insert("bare-assertion".to_owned(), result);
                observations.push(envelope.observe().expect("bare envelope shape"));
            }
            "duplicate" => {
                let mut envelope = observations[0].envelope().clone();
                envelope.evidence_id.push_str("-duplicate");
                envelope.method.push_str("-duplicate");
                envelope.producer.actor_id.push_str("-duplicate");
                let result = digest("duplicate-result");
                envelope.result = ObligationResult::Supported {
                    result_digest: result.clone(),
                };
                envelope
                    .outputs
                    .insert(envelope.obligation.id.clone(), result);
                envelope.artifact_digests = [digest("duplicate-artifact")].into_iter().collect();
                observations.push(envelope.observe().expect("duplicate obligation"));
            }
            "unknown" => {
                let mut envelope = observations.remove(0).envelope().clone();
                let prior_id = envelope.obligation.id.clone();
                envelope.obligation.id = "unknown-obligation".to_owned();
                let result = envelope.result.result_digest().to_owned();
                envelope.outputs.remove(&prior_id);
                envelope
                    .outputs
                    .insert(envelope.obligation.id.clone(), result);
                observations.push(envelope.observe().expect("unknown obligation"));
            }
            "shared" => {
                let artifact = observations[0]
                    .envelope()
                    .artifact_digests
                    .iter()
                    .next()
                    .expect("artifact")
                    .clone();
                let mut envelope = observations.remove(1).envelope().clone();
                envelope.artifact_digests = [artifact].into_iter().collect();
                observations.push(envelope.observe().expect("shared artifact"));
            }
            _ => unreachable!(),
        }
        let decision = decide(&mut ledger, &definitions, claim_id, observations);
        assert_eq!(decision.status, DecisionStatus::Rejected, "{case}");
        assert!(decision.reasons.iter().any(|reason| reason.contains(case)));
    }
}

#[test]
pub(crate) fn false_pass_controls_reject_report_only_results_and_contradictions() {
    let definitions = definitions();
    let claim_id = "CL-COMPLETION";
    for case in ["report-only", "contradicted"] {
        let mut ledger = super::super::scenario::semantic_model_ledger();
        pass_before(
            &mut ledger,
            &definitions,
            claim_id,
            &format!("{case}-prior"),
        );
        let mut observations = observations_for(&definitions, claim_id, case);
        let position = if case == "report-only" {
            observations
                .iter()
                .position(|item| {
                    item.envelope().obligation.kind == ObligationKind::FalsePassControl
                })
                .expect("false pass")
        } else {
            observations
                .iter()
                .position(|item| {
                    item.envelope().obligation.kind != ObligationKind::FalsePassControl
                })
                .expect("ordinary obligation")
        };
        let mut envelope = observations.remove(position).envelope().clone();
        let result_digest = envelope.result.result_digest().to_owned();
        envelope.result = if case == "report-only" {
            envelope.false_pass_model = None;
            ObligationResult::ObservedFailure {
                result_digest,
                failure_code: "arbitrary-self-report".to_owned(),
            }
        } else {
            ObligationResult::Contradicted {
                result_digest,
                reason: "live result disagreed".to_owned(),
            }
        };
        observations.push(envelope.observe().expect("result variant"));
        let decision = decide(&mut ledger, &definitions, claim_id, observations);
        assert_eq!(decision.status, DecisionStatus::Rejected);
        let expected_reason = if case == "report-only" {
            "report-only"
        } else {
            "contradicted"
        };
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| reason.contains(expected_reason))
        );
    }
}
