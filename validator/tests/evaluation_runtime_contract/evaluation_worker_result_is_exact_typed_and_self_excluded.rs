#[test]
fn evaluation_worker_result_is_exact_typed_and_self_excluded() {
    const RESULT_PATH: &str =
        "docs/ultragoal-successor-live/worker-results/EVALUATION-PRODUCTION-RUNTIME-086.json";
    // This historical worker record names its original source set; it is not
    // authority for the current existence of those source paths.
    const HISTORICAL_ARTIFACT_PATHS: [&str; 18] = [
        "fixtures/evaluation-engine/paired-valid.json",
        "fixtures/evaluation-engine/red-cases.json",
        "validator/src/cli/capture/fixture/mod.rs",
        "validator/src/cli/capture/fixture/execute.rs",
        "validator/src/cli/capture/fixture/permit.rs",
        "validator/src/evaluation/ledger.rs",
        "validator/src/evaluation/mod.rs",
        "validator/src/evaluation/promotion_ledger.rs",
        "validator/src/evaluation/records.rs",
        "validator/src/evaluation/research.rs",
        "validator/src/evaluation/runtime.rs",
        "validator/src/fixture_scheduler/mod.rs",
        "validator/src/fixture_scheduler/outcome.rs",
        "validator/src/fixture_scheduler/scheduler.rs",
        "validator/tests/evaluation_contract.rs",
        "validator/tests/evaluation_runtime_contract.rs",
        "validator/tests/fixture_scheduler_contract/execution_adapter/detached_descendant.rs",
        "validator/tests/fixture_scheduler_contract/execution_adapter/process_group.rs",
    ];
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let result_bytes = fs::read(repository.join(RESULT_PATH)).unwrap();
    let _typed = crate::orchestration::WorkerResultV1::parse_json(&result_bytes).unwrap();
    let result: Value = serde_json::from_slice(&result_bytes).unwrap();
    let object = result.as_object().unwrap();
    let expected_keys = BTreeSet::from([
        "artifacts",
        "base_state",
        "candidate_identity",
        "changes",
        "commands_and_tests",
        "context_id",
        "dependency_nodes",
        "effects",
        "final_state",
        "findings",
        "fixtures",
        "generated_outputs",
        "lease_id",
        "limitations",
        "no_claim_statement",
        "requested_root_changes",
        "requirements",
        "touched_paths",
        "touched_semantics",
        "unresolved_dependencies",
        "worker",
    ]);
    assert_eq!(
        object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        expected_keys
    );
    assert_eq!(
        result["worker"],
        "/root/evaluation_research_authority_repair_engineer"
    );
    assert_eq!(
        result["lease_id"],
        "EVALUATION-RESEARCH-AUTHORITY-BINDING-086-R7"
    );
    assert_eq!(
        result["no_claim_statement"],
        "This worker does not claim readiness, release, or completion."
    );
    assert_eq!(
        result["final_state"]["status"],
        "candidate_for_root_acceptance"
    );
    let lease_exact_ceiling = "This worker does not claim root adoption, public evaluation command availability, installed runtime behavior, representative product journeys, claim elevation, readiness, release, or completion.";
    assert_eq!(
        result["final_state"]["lease_exact_no_claim_statement"],
        lease_exact_ceiling
    );
    assert!(
        result["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation == lease_exact_ceiling)
    );
    assert_eq!(result["generated_outputs"], json!([RESULT_PATH]));
    assert_eq!(
        result["fixtures"],
        json!([
            "fixtures/evaluation-engine/paired-valid.json",
            "fixtures/evaluation-engine/red-cases.json"
        ])
    );

    let mut expected_touched = HISTORICAL_ARTIFACT_PATHS.to_vec();
    expected_touched.push(RESULT_PATH);
    assert_eq!(result["touched_paths"], json!(expected_touched));
    let rows = result["artifacts"].as_array().unwrap();
    assert_eq!(rows.len(), HISTORICAL_ARTIFACT_PATHS.len());
    let mut aggregate = Sha256::new();
    let mut aggregate_bytes = 0_u64;
    for (row, path) in rows.iter().zip(HISTORICAL_ARTIFACT_PATHS) {
        assert_eq!(row["path"], path);
        let file_sha256 = row["sha256"].as_str().expect("historical digest");
        assert!(file_sha256.strip_prefix("sha256:").is_some_and(
            |digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        ));
        let byte_length = row["byte_length"].as_u64().expect("historical byte length");
        aggregate.update(path.as_bytes());
        aggregate.update(b"\t");
        aggregate.update(file_sha256.trim_start_matches("sha256:").as_bytes());
        aggregate.update(b"\n");
        aggregate_bytes += byte_length;
    }
    let aggregate = format!("sha256:{:x}", aggregate.finalize());
    assert_eq!(
        result["candidate_identity"]["artifact_count"],
        HISTORICAL_ARTIFACT_PATHS.len() as u64
    );
    assert_eq!(
        result["candidate_identity"]["artifact_bytes"],
        aggregate_bytes
    );
    assert_eq!(
        result["candidate_identity"]["artifact_set_sha256"],
        aggregate
    );
    assert_eq!(result["candidate_identity"]["candidate_id"], aggregate);
    assert_eq!(result["final_state"]["source_candidate_id"], aggregate);
}
