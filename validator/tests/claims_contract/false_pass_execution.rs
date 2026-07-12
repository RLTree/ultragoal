use super::claims::{
    ActorRole, ClaimDefinitions, DecisionLedger, DecisionStatus, ExecutionObservation,
    FalsePassExecution, ObligationKind, ObligationResult, Observation, TestSubstitution,
    product_executor_catalog_preflight_for_test,
};
use super::support::{
    authority, candidate_id, context_id, definitions, digest, now, observations_for, pass_before,
    reviewer, submit,
};
use std::collections::BTreeMap;

const CLAIMS: [&str; 5] = [
    "CL-PACKAGE",
    "CL-INSTALL",
    "CL-ORCHESTRATION",
    "CL-RELEASE",
    "CL-COMPLETION",
];

fn false_pass_position(observations: &[Observation]) -> usize {
    observations
        .iter()
        .position(|item| item.envelope().obligation.kind == ObligationKind::FalsePassControl)
        .expect("false-pass control")
}

fn decide(
    ledger: &mut DecisionLedger,
    definitions: &ClaimDefinitions,
    claim_id: &str,
    observations: Vec<Observation>,
) -> super::claims::ClaimDecision {
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
fn test_transport_binds_receipts_to_exact_adopted_control_names() {
    let definitions = definitions();
    for claim_id in CLAIMS {
        let observations = observations_for(&definitions, claim_id, "authority-positive");
        for observation in observations
            .iter()
            .filter(|item| item.envelope().obligation.kind == ObligationKind::FalsePassControl)
        {
            let envelope = observation.envelope();
            let observed = envelope
                .false_pass_execution
                .as_ref()
                .expect("authority observation");
            let execution = observed.execution();
            assert!(observed.verify_integrity().is_ok());
            assert_eq!(envelope.result.result_digest(), observed.observed_digest());
            assert_eq!(execution.registry_digest(), definitions.registry_digest());
            assert_eq!(execution.claim_id(), claim_id);
            assert_eq!(execution.control_id(), envelope.obligation.id);
            assert!(
                execution
                    .expected_failure_contract()
                    .starts_with("claims-negative-control-rejected:")
            );
            assert_eq!(execution.exit_code(), 70);
            assert!(execution.control_definition_digest().starts_with("sha256:"));
            assert!(execution.command_spec_digest().starts_with("sha256:"));
            assert!(execution.argument_digest().starts_with("sha256:"));
            assert!(execution.script_digest().starts_with("sha256:"));
            assert!(execution.authority_nonce().starts_with("sha256:"));
            assert!(
                execution
                    .actual_causal_outcome()
                    .is_expected_failure(execution.expected_failure_contract())
            );
            assert_ne!(execution.executor().actor_id, observed.observer().actor_id);
            assert_ne!(execution.executor().actor_id, envelope.producer.actor_id);
            assert_ne!(observed.observer().actor_id, envelope.producer.actor_id);
            assert!(execution.started_at_unix_ms() < execution.ended_at_unix_ms());
            assert!(execution.ended_at_unix_ms() <= observed.observed_at_unix_ms());
        }
        let mut ledger = DecisionLedger::default();
        pass_before(
            &mut ledger,
            &definitions,
            claim_id,
            "authority-positive-prior",
        );
        assert_eq!(
            decide(&mut ledger, &definitions, claim_id, observations).status,
            DecisionStatus::Passed,
            "{claim_id}"
        );
    }
}

#[test]
fn accepted_receipt_fields_cannot_be_deserialized_or_omitted() {
    let definitions = definitions();
    for claim_id in CLAIMS {
        let observations = observations_for(&definitions, claim_id, "receipt-private");
        let observed = observations[false_pass_position(&observations)]
            .envelope()
            .false_pass_execution
            .as_ref()
            .expect("receipt");
        let execution_value = serde_json::to_value(observed.execution()).expect("encode execution");
        for field in execution_value
            .as_object()
            .expect("execution object")
            .keys()
        {
            let mut omitted = execution_value.clone();
            omitted
                .as_object_mut()
                .expect("execution object")
                .remove(field);
            assert!(
                serde_json::from_value::<FalsePassExecution>(omitted).is_err(),
                "{claim_id}:{field}"
            );
        }
        assert!(serde_json::from_value::<FalsePassExecution>(execution_value).is_err());
        let observation_value = serde_json::to_value(observed).expect("encode observation");
        for field in observation_value
            .as_object()
            .expect("observation object")
            .keys()
        {
            let mut omitted = observation_value.clone();
            omitted
                .as_object_mut()
                .expect("observation object")
                .remove(field);
            assert!(
                serde_json::from_value::<ExecutionObservation>(omitted).is_err(),
                "{claim_id}:{field}"
            );
        }
        assert!(serde_json::from_value::<ExecutionObservation>(observation_value).is_err());
    }
}

#[test]
fn producer_synthesis_regression_rejects_without_an_authority_receipt() {
    let definitions = definitions();
    for claim_id in CLAIMS {
        let mut ledger = DecisionLedger::default();
        pass_before(
            &mut ledger,
            &definitions,
            claim_id,
            "producer-synthesis-prior",
        );
        let mut observations = observations_for(&definitions, claim_id, "producer-synthesis");
        let position = false_pass_position(&observations);
        let mut envelope = observations.remove(position).envelope().clone();
        let expected_failure = envelope
            .false_pass_execution
            .as_ref()
            .expect("real authority observation")
            .execution()
            .expected_failure_contract()
            .to_owned();
        let fabricated = serde_json::json!({
            "execution_id": "fabricated-execution",
            "control_id": envelope.obligation.id,
            "negative_stimulus_digest": digest("fabricated-stimulus-never-applied"),
            "expected_failure_contract": expected_failure,
            "executor": {
                "actor_id": "fabricated-executor",
                "roles": ["control_executor"]
            },
            "execution_method": "fabricated-execution-method",
            "executor_tool_digest": digest("fabricated-tool-never-invoked"),
            "started_at_unix_ms": 90,
            "ended_at_unix_ms": 95,
            "live_context_id": context_id(),
            "candidate_id": candidate_id(),
            "max_age_ms": 60_000,
            "truth_surface": envelope.truth_surface,
            "declared_ceiling": envelope.declared_ceiling,
            "actual_causal_outcome": {
                "status": "executed_failure",
                "failure_code": expected_failure,
                "outcome_digest": digest("fabricated-outcome-without-execution")
            },
            "output_digests": { "fabricated-control": digest("fabricated-outcome-without-execution") },
            "artifact_digests": [digest("fabricated-artifact-never-created")]
        });
        assert!(serde_json::from_value::<FalsePassExecution>(fabricated).is_err());
        envelope.false_pass_execution = None;
        envelope.result = ObligationResult::ObservedFailure {
            result_digest: digest("fabricated-outcome-without-execution"),
            failure_code: "fabricated-failure".to_owned(),
        };
        envelope.outputs = BTreeMap::from([(
            envelope.obligation.id.clone(),
            envelope.result.result_digest().to_owned(),
        )]);
        observations.push(envelope.observe().expect("report-only envelope"));
        let decision = decide(&mut ledger, &definitions, claim_id, observations);
        assert_eq!(decision.status, DecisionStatus::Rejected, "{claim_id}");
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| reason.contains("report-only") || reason.contains("unobserved")),
            "{claim_id}:{:?}",
            decision.reasons
        );
    }
}

#[test]
fn envelope_substitution_stale_cross_context_cross_candidate_and_report_only_reject() {
    let definitions = definitions();
    for claim_id in CLAIMS {
        for case in [
            "result",
            "input",
            "tool",
            "stale",
            "context",
            "candidate",
            "surface",
            "ceiling",
            "self-author",
            "report-only",
        ] {
            let mut ledger = DecisionLedger::default();
            pass_before(
                &mut ledger,
                &definitions,
                claim_id,
                &format!("{case}-prior"),
            );
            let mut observations = observations_for(&definitions, claim_id, case);
            let position = false_pass_position(&observations);
            let mut envelope = observations.remove(position).envelope().clone();
            match case {
                "result" => {
                    envelope.result = ObligationResult::Supported {
                        result_digest: digest("substituted-result"),
                    }
                }
                "input" => {
                    envelope.inputs = BTreeMap::from([(
                        envelope.obligation.id.clone(),
                        digest("substituted-negative-stimulus"),
                    )])
                }
                "tool" => {
                    envelope.environment_and_tools = BTreeMap::from([(
                        "substituted-method".to_owned(),
                        digest("substituted-tool"),
                    )])
                }
                "stale" => {
                    envelope.observed_at_unix_ms = 1;
                    envelope.max_age_ms = 1;
                }
                "context" => envelope.live_context_id = "wrong-context".to_owned(),
                "candidate" => envelope.candidate_id = "wrong-candidate".to_owned(),
                "surface" => envelope.truth_surface = "wrong-surface".to_owned(),
                "ceiling" => envelope.declared_ceiling = "wrong-ceiling".to_owned(),
                "self-author" => {
                    envelope.producer.actor_id = reviewer(&definitions, claim_id).actor_id;
                    envelope.producer.roles.insert(ActorRole::MaterialScorer);
                }
                "report-only" => {
                    envelope.false_pass_execution = None;
                    envelope.result = ObligationResult::ObservedFailure {
                        result_digest: digest("report-only"),
                        failure_code: "report-only".to_owned(),
                    };
                }
                _ => unreachable!(),
            }
            envelope.outputs = BTreeMap::from([(
                envelope.obligation.id.clone(),
                envelope.result.result_digest().to_owned(),
            )]);
            observations.push(envelope.observe().expect("mutated envelope"));
            assert_eq!(
                decide(&mut ledger, &definitions, claim_id, observations).status,
                DecisionStatus::Rejected,
                "{claim_id}:{case}"
            );
        }
    }
}

#[test]
fn receipt_replay_is_rejected_after_a_real_control_execution() {
    let definitions = definitions();
    let claim_id = "CL-PACKAGE";
    let mut ledger = DecisionLedger::default();
    pass_before(&mut ledger, &definitions, claim_id, "receipt-replay-prior");
    let mut observations = observations_for(&definitions, claim_id, "receipt-replay");
    let position = false_pass_position(&observations);
    let mut duplicate = observations[position].envelope().clone();
    duplicate.evidence_id.push_str("-duplicate");
    duplicate.method.push_str("-duplicate");
    duplicate.producer.actor_id.push_str("-duplicate");
    duplicate.artifact_digests = [digest("duplicate-envelope-artifact")]
        .into_iter()
        .collect();
    observations.push(duplicate.observe().expect("duplicate envelope"));
    let decision = decide(&mut ledger, &definitions, claim_id, observations);
    assert_eq!(decision.status, DecisionStatus::Rejected);
    assert!(
        decision
            .reasons
            .iter()
            .any(|reason| reason.contains("execution-replayed"))
    );
}

#[test]
fn test_transport_indexes_every_adopted_claims_exact_control_set() {
    let definitions = definitions();
    let authority = authority(&definitions);
    for claim_id in definitions.order() {
        let expected = definitions
            .definition(claim_id)
            .expect("adopted definition")
            .false_pass_controls
            .iter()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        let observed = authority
            .execute_claim(claim_id)
            .expect("closed named controls execute");
        assert_eq!(
            observed
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>(),
            expected
        );
        for execution in observed.values().map(ExecutionObservation::execution) {
            assert_eq!(execution.registry_digest(), definitions.registry_digest());
            assert_eq!(execution.claim_id(), claim_id);
            assert_eq!(
                execution.truth_surface(),
                definitions.definition(claim_id).unwrap().truth_surface
            );
            assert_eq!(
                execution.declared_ceiling(),
                definitions
                    .definition(claim_id)
                    .unwrap()
                    .allowed_ceiling_on_pass
            );
        }
    }
}

#[test]
fn generic_tool_argument_and_fixture_spec_substitution_fail_before_receipt_issue() {
    let definitions = definitions();
    let authority = authority(&definitions);
    for claim_id in definitions.order() {
        let control_id = definitions
            .definition(claim_id)
            .unwrap()
            .false_pass_controls
            .first()
            .unwrap();
        assert!(
            authority
                .execute_substituted_for_test(claim_id, control_id, TestSubstitution::GenericTool,)
                .is_err(),
            "generic failure substitution:{claim_id}"
        );
    }
    let claim_id = "CL-SOURCE";
    let control_id = definitions
        .definition(claim_id)
        .unwrap()
        .false_pass_controls
        .first()
        .unwrap();
    for substitution in [TestSubstitution::Arguments, TestSubstitution::FixtureSpec] {
        assert!(
            authority
                .execute_substituted_for_test(claim_id, control_id, substitution)
                .is_err(),
            "{substitution:?}"
        );
    }
}

#[test]
fn production_preflight_rejects_test_transport_as_product_behavior() {
    assert_eq!(
        product_executor_catalog_preflight_for_test(),
        Err("claims-control-product-executor-catalog-unavailable".to_owned())
    );
}

#[test]
fn authority_uses_only_current_digest_candidate_bindings_and_unique_executions() {
    let definitions = std::sync::Arc::new(definitions());
    let bound_authority = authority(&definitions);
    assert!(bound_authority.context_id().starts_with("sha256:"));
    assert!(bound_authority.candidate_id().starts_with("sha256:"));
    assert_ne!(bound_authority.context_id(), "arbitrary-context");
    assert_ne!(bound_authority.candidate_id(), "arbitrary-candidate");
    drop(bound_authority);

    let handles = (0..4)
        .map(|_| {
            let definitions = std::sync::Arc::clone(&definitions);
            std::thread::spawn(move || {
                authority(&definitions)
                    .execute_claim("CL-SOURCE")
                    .expect("concurrent named controls")
                    .into_values()
                    .map(|observed| observed.execution().execution_id().to_owned())
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let mut ids = std::collections::BTreeSet::new();
    for handle in handles {
        for id in handle.join().expect("authority thread") {
            assert!(ids.insert(id), "execution ids are unique under concurrency");
        }
    }
    assert_eq!(ids.len(), 20);
}

#[test]
fn cross_claim_named_control_receipt_rejects() {
    let definitions = definitions();
    let mut source = observations_for(&definitions, "CL-SOURCE", "cross-claim-source");
    let source_position = false_pass_position(&source);
    let foreign = source
        .remove(source_position)
        .envelope()
        .false_pass_execution
        .as_ref()
        .expect("source control")
        .clone();
    let claim_id = "CL-PACKAGE";
    let mut ledger = DecisionLedger::default();
    pass_before(&mut ledger, &definitions, claim_id, "cross-claim-prior");
    let mut package = observations_for(&definitions, claim_id, "cross-claim");
    let position = false_pass_position(&package);
    let mut envelope = package.remove(position).envelope().clone();
    envelope.false_pass_execution = Some(foreign);
    envelope.result = ObligationResult::Supported {
        result_digest: envelope
            .false_pass_execution
            .as_ref()
            .unwrap()
            .observed_digest()
            .to_owned(),
    };
    envelope.outputs = BTreeMap::from([(
        envelope.obligation.id.clone(),
        envelope.result.result_digest().to_owned(),
    )]);
    package.push(envelope.observe().expect("cross-claim envelope shape"));
    assert_eq!(
        decide(&mut ledger, &definitions, claim_id, package).status,
        DecisionStatus::Rejected
    );
}
