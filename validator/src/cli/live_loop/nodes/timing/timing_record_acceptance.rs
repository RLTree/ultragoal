use super::{NODE_TIMING_REL, read_current};
use serde_json::{Value, json};

#[path = "observation_fixture.rs"]
mod observation_fixture;
#[path = "test_rows.rs"]
mod test_rows;
use self::observation_fixture::write_command_observation;
use self::test_rows::{current_timing_row, digest, expected_output_digest, expected_result_digest};

#[test]
fn node_timing_reader_accepts_non_lane_verified_cache_hit_with_equivalence() {
    let timings = read_with_patch(json!({
        "proof_kind": "verified_cache_hit",
        "baseline_proof_kind": "verified_baseline_reuse",
        "baseline_invalidation_proof": "baseline_reused_from_verified_current_input_timing_row",
        "cache_hit": true,
        "work_unit_count": 0,
        "equivalence_status": "verified_same_candidate_cache_replay",
        "invalidation_proof": "cache_key_current_input_digest_command_contract_runtime_model_versions_and_environment_matched",
        "prior_result_digest": expected_result_digest(0, false, &expected_output_digest()),
        "replayed_output_digest": expected_output_digest(),
        "cache_equivalence_status": "pass"
    }));

    assert_eq!(timings, 1);
}

#[test]
fn node_timing_reader_rejects_proof_shaped_rows_without_current_work_or_equivalence() {
    let cases = [
        json!({"actual_work_duration_ms": 3}),
        json!({"telemetry_reconciliation_duration_ms": serde_json::Value::Null}),
        json!({"reconciled_command_duration_ms": serde_json::Value::Null}),
        json!({"reconciled_command_duration_ms": 5}),
        json!({"product_latency_ms": serde_json::Value::Null}),
        json!({"product_latency_ms": 5}),
        json!({"validator_version": ""}),
        json!({"runtime_execution_model": ""}),
        json!({"execution_task_class": ""}),
        json!({"execution_task_class": "shared_authority_write_serial"}),
        json!({"execution_serial_reason": "hidden serial cap"}),
        json!({"worker_count": 0}),
        json!({"worker_count": 2}),
        json!({"task_count": 0}),
        json!({"queue_depth": 0}),
        json!({"task_count": 2, "queue_depth": 3}),
        json!({"worker_state": ""}),
        json!({"task_state": ""}),
        json!({"queue_state": ""}),
        json!({"executor_behavior": ""}),
        json!({"executor_scope": ""}),
        json!({"parallel_write_policy": "shared_validation_artifact_write_allowed"}),
        json!({"verified_local_command": ""}),
        json!({"command_argv": []}),
        json!({"exit_status": serde_json::Value::Null}),
        json!({"receipt_paths": [], "artifact_paths": []}),
        json!({"result_digest": serde_json::Value::Null}),
        json!({"output_digest": serde_json::Value::Null}),
        json!({"result_digest": digest("different-result")}),
        json!({"output_digest": digest("different-output")}),
        forged_digest_pair_patch(),
        json!({"verified_local_result_digest": "sha256:short"}),
        json!({"verified_local_output_digest": "sha256:short"}),
        json!({"verified_local_stdout_digest": "sha256:short"}),
        json!({"claim_ceiling": "readiness_overclaim"}),
        json!({"claim_impact": "supports_readiness"}),
        json!({"timing_status": "pass", "failure_class": "live_loop_speedup_target_missed"}),
        json!({"timing_status": "pass", "telemetry_reconciliation_status": "missing"}),
        json!({"equivalence_status": "unknown"}),
        json!({"invalidation_proof": ""}),
        json!({"cache_hit": true}),
        json!({"work_unit_count": 0}),
        json!({"proof_kind": "verified_cache_hit", "cache_hit": true}),
        json!({"proof_kind": "verified_cache_hit", "cache_hit": true, "work_unit_count": 0, "equivalence_status": "verified_same_candidate_cache_replay", "prior_result_digest": digest("prior"), "replayed_output_digest": digest("output"), "cache_equivalence_status": "pass"}),
        json!({"proof_kind": "verified_cache_hit", "cache_hit": true, "work_unit_count": 0, "equivalence_status": "verified_same_candidate_cache_replay", "prior_result_digest": digest("result"), "replayed_output_digest": digest("replayed"), "cache_equivalence_status": "pass"}),
        json!({"proof_kind": "verified_cache_hit", "cache_hit": true, "work_unit_count": 0, "equivalence_status": "verified_same_candidate_cache_replay", "prior_result_digest": digest("result"), "replayed_output_digest": digest("output"), "cache_equivalence_status": "miss"}),
        json!({"proof_kind": "planned"}),
    ];

    for patch in cases {
        assert_eq!(read_with_patch(patch), 0);
    }
}

#[test]
fn measurement_rust_test_zero_or_missing_count_is_rejected_by_timing_reader() {
    assert_eq!(
        read_measurement_rust_test_with_count(json!(0)),
        0,
        "zero-test pass rows must not project validation or speed support"
    );
    assert_eq!(
        read_measurement_rust_test_with_count(serde_json::Value::Null),
        0,
        "missing test counts must not project validation or speed support"
    );
    assert_eq!(
        read_measurement_rust_test_with_count(json!(114)),
        0,
        "local-only command receipts must not project Lane 014 validation or speed support"
    );
}

#[test]
fn node_timing_reader_accepts_non_lane_batch_derived_measurement_queue_state() {
    assert_eq!(
        read_with_patch(json!({"task_count": 3, "queue_depth": 2})),
        1
    );
}

fn read_with_patch(patch: Value) -> usize {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-timing-record-acceptance",
    );
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context = crate::digest::bytes(
        format!(
            "validator={};law={};schema={};fixture={};tier=hot;cache=verified-local",
            crate::cli::live_loop::graph::validator_version(),
            crate::cli::live_loop::graph::law_version(),
            crate::cli::live_loop::graph::schema_version(),
            crate::cli::live_loop::graph::fixture_version()
        )
        .as_bytes(),
    );
    let changed_inputs =
        crate::cli::live_loop::changed_inputs::ChangedInputs::for_tests(&changed, &context);
    let input = super::graph::surface_input_digest(
        super::surface_by_id("fmt_check").expect("fmt surface"),
        candidate,
        &changed,
        &context,
    );
    let mut row = current_timing_row(candidate, &input, "pass", "none");
    write_command_observation(&root, &row);
    let object = row.as_object_mut().expect("current timing row object");
    for (key, value) in patch.as_object().expect("patch object") {
        object.insert(key.clone(), value.clone());
    }
    crate::json_boundary::write_json(&root.join(NODE_TIMING_REL), &json!({"nodes": [row]}))
        .expect("timing artifact");
    let timings = read_current(&root, candidate, "hot", "verified-local", &changed_inputs);
    std::fs::remove_dir_all(root).expect("cleanup timing acceptance");
    timings.len()
}

fn read_measurement_rust_test_with_count(test_count: Value) -> usize {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-measurement-rust-test-count",
    );
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context = crate::digest::bytes(
        format!(
            "validator={};law={};schema={};fixture={};tier=hot;cache=verified-local",
            crate::cli::live_loop::graph::validator_version(),
            crate::cli::live_loop::graph::law_version(),
            crate::cli::live_loop::graph::schema_version(),
            crate::cli::live_loop::graph::fixture_version()
        )
        .as_bytes(),
    );
    let changed_inputs =
        crate::cli::live_loop::changed_inputs::ChangedInputs::for_tests(&changed, &context);
    let surface = super::surface_by_id("live_loop_measurement_rust_tests")
        .expect("measurement rust test surface");
    let input = super::graph::surface_input_digest(surface, candidate, &changed, &context);
    let mut row = current_timing_row(candidate, &input, "pass", "none");
    let patch = measurement_rust_test_patch(test_count);
    let object = row.as_object_mut().expect("current timing row object");
    for (key, value) in patch.as_object().expect("patch object") {
        object.insert(key.clone(), value.clone());
    }
    write_command_observation(&root, &row);
    crate::json_boundary::write_json(&root.join(NODE_TIMING_REL), &json!({"nodes": [row]}))
        .expect("timing artifact");
    let timings = read_current(&root, candidate, "hot", "verified-local", &changed_inputs);
    std::fs::remove_dir_all(root).expect("cleanup measurement rust test count");
    timings.len()
}

fn forged_digest_pair_patch() -> Value {
    let output_digest = digest("self-consistent-wrong-output");
    let result_digest = digest("self-consistent-wrong-result");
    json!({
        "output_digest": output_digest,
        "verified_local_output_digest": output_digest,
        "result_digest": result_digest,
        "verified_local_result_digest": result_digest
    })
}

fn measurement_rust_test_patch(test_count: Value) -> Value {
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context = crate::digest::bytes(
        format!(
            "validator={};law={};schema={};fixture={};tier=hot;cache=verified-local",
            crate::cli::live_loop::graph::validator_version(),
            crate::cli::live_loop::graph::law_version(),
            crate::cli::live_loop::graph::schema_version(),
            crate::cli::live_loop::graph::fixture_version()
        )
        .as_bytes(),
    );
    let surface = super::surface_by_id("live_loop_measurement_rust_tests")
        .expect("measurement rust test surface");
    let input = super::graph::surface_input_digest(surface, candidate, &changed, &context);
    let cache_key =
        super::graph::verified_local_cache_key(surface, &input, "hot", "verified-local");
    json!({
        "node_id": "live_loop_measurement_rust_tests",
        "input_digest": input,
        "current_input_digest": input,
        "canonical_full_command": surface.canonical_full_command,
        "verified_local_command": surface.canonical_full_command,
        "command_argv": ["cargo", "test", "--offline", "live_loop::nodes::measurement", "--lib", "--quiet"],
        "verified_local_command_argv": ["cargo", "test", "--offline", "live_loop::nodes::measurement", "--lib", "--quiet"],
        "cache_key": cache_key,
        "graph_task_class": surface.execution_task_class.id(),
        "execution_task_class": surface.execution_task_class.id(),
        "execution_serial_reason": surface.execution_serial_reason,
        "verified_local_executed_test_count": test_count
    })
}
