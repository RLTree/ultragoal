use super::claims::{
    ActorRole, ClaimDefinitions, DecisionLedger, DecisionStatus, ObligationKind, ObligationResult,
    Observation, PRIVATE_TRANSPORT_UNAVAILABLE, SemanticControlModel, SemanticControlObservation,
    TestSubstitution, product_executor_catalog_preflight_for_test,
};
use super::support::{
    authority, candidate_id, context_id, control_scratch_root, definitions, digest, now,
    observations_for, pass_before, reviewer, submit,
};
use std::collections::BTreeMap;

fn scratch_snapshot(root: &std::path::Path) -> Vec<(std::path::PathBuf, String, Vec<u8>)> {
    fn visit(
        root: &std::path::Path,
        path: &std::path::Path,
        rows: &mut Vec<(std::path::PathBuf, String, Vec<u8>)>,
    ) {
        let metadata = std::fs::symlink_metadata(path).expect("scratch metadata");
        let relative = path
            .strip_prefix(root)
            .expect("scratch relative")
            .to_owned();
        if metadata.file_type().is_symlink() {
            rows.push((
                relative,
                "symlink".to_owned(),
                std::fs::read_link(path)
                    .expect("scratch symlink target")
                    .as_os_str()
                    .as_encoded_bytes()
                    .to_vec(),
            ));
        } else if metadata.is_dir() {
            rows.push((relative, "directory".to_owned(), Vec::new()));
            let mut children = std::fs::read_dir(path)
                .expect("scratch read dir")
                .map(|entry| entry.expect("scratch entry").path())
                .collect::<Vec<_>>();
            children.sort();
            for child in children {
                visit(root, &child, rows);
            }
        } else if metadata.is_file() {
            rows.push((
                relative,
                "file".to_owned(),
                std::fs::read(path).expect("scratch file"),
            ));
        } else {
            rows.push((relative, "special".to_owned(), Vec::new()));
        }
    }

    let mut rows = Vec::new();
    visit(root, root, &mut rows);
    rows
}

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
fn sealed_semantic_model_binds_exact_adopted_control_names_without_process_proof() {
    let definitions = definitions();
    for claim_id in CLAIMS {
        let observations = observations_for(&definitions, claim_id, "authority-positive");
        for observation in observations
            .iter()
            .filter(|item| item.envelope().obligation.kind == ObligationKind::FalsePassControl)
        {
            let envelope = observation.envelope();
            let observed = envelope
                .false_pass_model
                .as_ref()
                .expect("semantic control model");
            let model = observed.model();
            assert!(observed.verify_integrity().is_ok());
            assert_eq!(envelope.result.result_digest(), observed.observed_digest());
            assert_eq!(model.registry_digest(), definitions.registry_digest());
            assert_eq!(model.claim_id(), claim_id);
            assert_eq!(model.control_id(), envelope.obligation.id);
            assert!(
                model
                    .expected_failure_contract()
                    .starts_with("claims-negative-control-rejected:")
            );
            assert!(model.control_definition_digest().starts_with("sha256:"));
            assert!(model.model_spec_digest().starts_with("sha256:"));
            assert!(model.authority_nonce().starts_with("sha256:"));
            assert!(model.model_implementation_digest().starts_with("sha256:"));
            assert!(model.modeled_result_digest().starts_with("sha256:"));
            assert!(model.model_record_digest().starts_with("sha256:"));
            assert_ne!(model.modeler().actor_id, observed.observer().actor_id);
            assert_ne!(model.modeler().actor_id, envelope.producer.actor_id);
            assert_ne!(observed.observer().actor_id, envelope.producer.actor_id);
            assert!(model.modeled_at_unix_ms() <= observed.observed_at_unix_ms());
        }
        let mut ledger = super::support::semantic_model_ledger();
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
fn default_product_ledger_rejects_a_well_formed_semantic_model_as_non_executed_proof() {
    let definitions = definitions();
    let claim_id = "CL-SOURCE";
    let mut ledger = DecisionLedger::default();
    let ids = submit(
        &mut ledger,
        observations_for(&definitions, claim_id, "default-ledger-refusal"),
    );
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
            .any(|reason| { reason == "claims-false-pass-semantic-model-not-executed-proof" })
    );
}

#[test]
fn semantic_model_fields_cannot_be_deserialized_omitted_or_mistaken_for_process_proof() {
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
fn producer_synthesis_regression_rejects_without_a_semantic_model() {
    let definitions = definitions();
    for claim_id in CLAIMS {
        let mut ledger = super::support::semantic_model_ledger();
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
            let mut ledger = super::support::semantic_model_ledger();
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

#[test]
fn model_replay_is_rejected_for_the_sealed_semantic_model() {
    let definitions = definitions();
    let claim_id = "CL-PACKAGE";
    let mut ledger = super::support::semantic_model_ledger();
    pass_before(&mut ledger, &definitions, claim_id, "model-replay-prior");
    let mut observations = observations_for(&definitions, claim_id, "model-replay");
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
            .any(|reason| reason.contains("model-replayed"))
    );
}

#[test]
fn sealed_semantic_model_indexes_every_adopted_claims_exact_control_set() {
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
            .model_claim_for_decision_tests(claim_id)
            .expect("sealed semantic controls model");
        assert_eq!(
            observed
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>(),
            expected
        );
        for model in observed.values().map(SemanticControlObservation::model) {
            assert_eq!(model.registry_digest(), definitions.registry_digest());
            assert_eq!(model.claim_id(), claim_id);
            assert_eq!(
                model.truth_surface(),
                definitions.definition(claim_id).unwrap().truth_surface
            );
            assert_eq!(
                model.declared_ceiling(),
                definitions
                    .definition(claim_id)
                    .unwrap()
                    .allowed_ceiling_on_pass
            );
        }
    }
}

#[test]
fn generic_tool_argument_and_fixture_spec_substitution_fail_before_model_or_proof_issue() {
    let definitions = definitions();
    let authority = authority(&definitions);
    for claim_id in definitions.order() {
        let control_id = definitions
            .definition(claim_id)
            .unwrap()
            .false_pass_controls
            .first()
            .unwrap();
        assert_eq!(
            authority
                .execute_substituted_for_test(claim_id, control_id, TestSubstitution::GenericTool,)
                .unwrap_err(),
            PRIVATE_TRANSPORT_UNAVAILABLE,
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
        assert_eq!(
            authority
                .execute_substituted_for_test(claim_id, control_id, substitution)
                .unwrap_err(),
            PRIVATE_TRANSPORT_UNAVAILABLE,
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
fn production_claims_module_compiles_without_dormant_process_transport() {
    let source = include_str!("../../src/claims/false_pass.rs");
    for prohibited in [
        "FixtureScheduler",
        "FixtureExecutor",
        "ConfinementPlan",
        "ProcessCapture",
        "RunDisposition",
        "execute_control",
        "run_substituted_fixture",
        "allow(unreachable_code)",
    ] {
        assert!(!source.contains(prohibited), "dormant transport:{prohibited}");
    }
    let library = include_str!("../../src/lib.rs");
    assert!(library.lines().any(|line| line.trim() == "mod claims;"));
}

#[test]
fn private_transport_refuses_before_schedule_model_or_proof_issue() {
    let definitions = definitions();
    let authority = authority(&definitions);
    let before = scratch_snapshot(control_scratch_root());

    assert_eq!(
        authority.execute_claim("CL-SOURCE").unwrap_err(),
        PRIVATE_TRANSPORT_UNAVAILABLE
    );

    let after = scratch_snapshot(control_scratch_root());
    assert_eq!(after, before);
}

#[test]
fn semantic_model_uses_current_digest_candidate_bindings_and_unique_model_ids() {
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
                    .model_claim_for_decision_tests("CL-SOURCE")
                    .expect("concurrent sealed semantic model")
                    .into_values()
                    .map(|observed| observed.model().model_id().to_owned())
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let mut ids = std::collections::BTreeSet::new();
    for handle in handles {
        for id in handle.join().expect("authority thread") {
            assert!(ids.insert(id), "model ids are unique under concurrency");
        }
    }
    assert_eq!(ids.len(), 20);
}

#[test]
fn cross_claim_named_control_model_rejects() {
    let definitions = definitions();
    let mut source = observations_for(&definitions, "CL-SOURCE", "cross-claim-source");
    let source_position = false_pass_position(&source);
    let foreign = source
        .remove(source_position)
        .envelope()
        .false_pass_model
        .as_ref()
        .expect("source control")
        .clone();
    let claim_id = "CL-PACKAGE";
    let mut ledger = super::support::semantic_model_ledger();
    pass_before(&mut ledger, &definitions, claim_id, "cross-claim-prior");
    let mut package = observations_for(&definitions, claim_id, "cross-claim");
    let position = false_pass_position(&package);
    let mut envelope = package.remove(position).envelope().clone();
    envelope.false_pass_model = Some(foreign);
    envelope.result = ObligationResult::Supported {
        result_digest: envelope
            .false_pass_model
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
