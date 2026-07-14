fn subject() -> (LeaseSpec, WorkPackage, WorkerResultV1) {
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    (lease, package, result)
}

#[test]
fn typed_result_fields_match_the_adopted_worker_result_v1_schema() {
    let contract = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/IMPLEMENTATION_DEPENDENCY_GRAPH.json",
    );
    let graph: Value = serde_json::from_slice(&std::fs::read(contract).unwrap()).unwrap();
    let schema = &graph["worker_output_schema"];
    assert_eq!(schema["$id"], "urn:harness-ultragoal:WorkerResult-v1");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["no_claim_statement"]["const"],
        "This worker does not claim readiness, release, or completion."
    );
    let (_, _, result) = subject();
    let keys: BTreeSet<_> = serde_json::to_value(result)
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    let required: BTreeSet<_> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(keys, required);
}

#[test]
fn congruent_worker_result_validates_and_hashes_deterministically() {
    let (lease, package, result) = subject();
    result.validate_for(&lease, &package).unwrap();
    assert_eq!(result.result_id().unwrap(), result.result_id().unwrap());
}

#[test]
fn parser_rejects_unknown_top_level_fields() {
    let (_, _, result) = subject();
    let mut value = serde_json::to_value(result).unwrap();
    value["surprise"] = json!(true);
    assert_eq!(
        WorkerResultV1::parse_json(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
}

#[test]
fn forged_worker_and_stale_candidate_fail_closed() {
    let (lease, package, mut result) = subject();
    result.worker = "forged-worker".to_owned();
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::StaleBinding
    );

    let (_, _, mut result) = subject();
    result
        .candidate_identity
        .insert("candidate_id".to_owned(), json!(digest('e')));
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::StaleBinding
    );
}

#[test]
fn path_and_semantic_scope_escape_fail_closed() {
    let (lease, package, mut result) = subject();
    result.touched_paths = vec!["validator/src/orchestration.rs".to_owned()];
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::UnknownScope
    );

    let (_, _, mut result) = subject();
    result.touched_semantics = vec!["orchestration::other".to_owned()];
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::UnknownScope
    );
}

#[test]
fn duplicate_generated_output_is_rejected() {
    let (lease, package, mut result) = subject();
    result
        .generated_outputs
        .push(result.generated_outputs[0].clone());
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::DuplicateOutput
    );
}

#[test]
fn generated_fixture_and_artifact_paths_must_be_declared_touched() {
    let (lease, package, mut result) = subject();
    result
        .touched_paths
        .retain(|path| path != "generated/orchestration/node_a.json");
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );

    let (_, _, mut result) = subject();
    result
        .touched_paths
        .retain(|path| path != "validator/src/orchestration/node_a.rs");
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
}

#[test]
fn performed_effect_escalation_is_rejected() {
    let (lease, package, mut result) = subject();
    result.effects[0] = EffectUse {
        class: EffectClass::Network,
        target: "focused-test".to_owned(),
        performed: true,
    };
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::EffectDenied
    );
}

#[test]
fn malformed_artifact_digest_is_rejected() {
    let (lease, package, mut result) = subject();
    result.artifacts[0].sha256 = "not-a-digest".to_owned();
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidDigest
    );
}

#[test]
fn exact_no_claim_statement_is_required() {
    let (lease, package, mut result) = subject();
    result.no_claim_statement = "ready".to_owned();
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
}

#[test]
fn worker_cannot_encode_a_completion_status() {
    let (lease, package, mut result) = subject();
    result
        .final_state
        .insert("status".to_owned(), json!("complete"));
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
}

#[test]
fn empty_or_outcomeless_test_records_cannot_false_pass() {
    let (lease, package, mut result) = subject();
    result.commands_and_tests = vec![Default::default()];
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );

    let (_, _, mut result) = subject();
    result.commands_and_tests[0].remove("status");
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
}
