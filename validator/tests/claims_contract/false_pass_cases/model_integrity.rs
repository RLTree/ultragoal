use super::*;

#[test]
pub(crate) fn semantic_model_fields_cannot_be_deserialized_omitted_or_mistaken_for_process_proof() {
    let definitions = definitions();
    for claim_id in CLAIMS {
        let observations = observations_for(&definitions, claim_id, "model-private");
        let observed = observations[false_pass_position(&observations)]
            .envelope()
            .false_pass_model
            .as_ref()
            .expect("semantic model");
        let model_value = serde_json::to_value(observed.model()).expect("encode semantic model");
        let model_object = model_value.as_object().expect("semantic model object");
        for prohibited in [
            "exit_code",
            "captured_process_digest",
            "executor",
            "execution_method",
            "started_at_unix_ms",
            "ended_at_unix_ms",
            "actual_causal_outcome",
            "output_digests",
            "artifact_digests",
        ] {
            assert!(
                !model_object.contains_key(prohibited),
                "{claim_id}:{prohibited}"
            );
        }
        for field in model_value
            .as_object()
            .expect("semantic model object")
            .keys()
        {
            let mut omitted = model_value.clone();
            omitted
                .as_object_mut()
                .expect("semantic model object")
                .remove(field);
            assert!(
                serde_json::from_value::<SemanticControlModel>(omitted).is_err(),
                "{claim_id}:{field}"
            );
        }
        assert!(serde_json::from_value::<SemanticControlModel>(model_value).is_err());
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
                serde_json::from_value::<SemanticControlObservation>(omitted).is_err(),
                "{claim_id}:{field}"
            );
        }
        assert!(serde_json::from_value::<SemanticControlObservation>(observation_value).is_err());
    }
}

#[test]
pub(crate) fn producer_synthesis_regression_rejects_without_a_semantic_model() {
    let definitions = definitions();
    for claim_id in CLAIMS {
        let mut ledger = super::super::scenario::semantic_model_ledger();
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
            .false_pass_model
            .as_ref()
            .expect("sealed semantic model")
            .model()
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
        assert!(serde_json::from_value::<SemanticControlModel>(fabricated).is_err());
        envelope.false_pass_model = None;
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
pub(crate) fn envelope_substitution_stale_cross_context_cross_candidate_and_report_only_reject() {
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
            let mut ledger = super::super::scenario::semantic_model_ledger();
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
                    envelope.false_pass_model = None;
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
