use super::{NODE_TIMING_REL, read_current};
use serde_json::json;

#[test]
fn node_timing_reader_accepts_only_current_candidate_and_input() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-node-timing");
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context =
        crate::digest::bytes(format!("{candidate}:hot:verified-local:{changed}").as_bytes());
    let input = super::graph::surface_input_digest(
        super::surface_by_id("fmt_check").expect("fmt surface"),
        candidate,
        &changed,
        &context,
    );
    crate::json_boundary::write_json(
        &root.join(NODE_TIMING_REL),
        &json!({
            "nodes": [
                current_timing_row(candidate, &input, "pass", "none"),
                {
                    "node_id": "build_check",
                    "candidate_digest": "sha256:stale",
                    "tier": "hot",
                    "cache_mode": "verified-local",
                    "input_digest": "sha256:wrong",
                    "canonical_full_command": "cargo build --offline --bin ultragoal --quiet",
                    "timing_status": "pass",
                    "cache_honesty": "pass",
                    "failure_class": "none",
                    "baseline_duration_ms": 1000,
                    "verified_local_duration_ms": 2
                }
            ]
        }),
    )
    .expect("timing artifact");

    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed,
        &context,
    );
    assert_eq!(timings.len(), 1);
    assert_eq!(
        timings["fmt_check"].affected_set_status,
        "clean_worktree_no_affected_files"
    );
    assert_eq!(timings["fmt_check"].timing_status, "pass");
    assert_eq!(timings["fmt_check"].failure_class, "none");
    assert_eq!(timings["fmt_check"].baseline_exit_code, Some(0));
    std::fs::remove_dir_all(root).expect("cleanup node timing");
}

#[test]
fn node_timing_reader_preserves_current_failed_measurement_rows() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-failed-timing");
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context =
        crate::digest::bytes(format!("{candidate}:hot:verified-local:{changed}").as_bytes());
    let input = super::graph::surface_input_digest(
        super::surface_by_id("fmt_check").expect("fmt surface"),
        candidate,
        &changed,
        &context,
    );
    crate::json_boundary::write_json(
        &root.join(NODE_TIMING_REL),
        &json!({
            "nodes": [
                current_timing_row(candidate, &input, "fail", "canonical_full_command_failed")
            ]
        }),
    )
    .expect("timing artifact");

    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed,
        &context,
    );
    assert_eq!(timings.len(), 1);
    assert_eq!(timings["fmt_check"].timing_status, "fail");
    assert_eq!(
        timings["fmt_check"].failure_class,
        "canonical_full_command_failed"
    );
    assert_eq!(timings["fmt_check"].baseline_exit_code, Some(101));
    std::fs::remove_dir_all(root).expect("cleanup failed timing");
}

#[test]
fn node_timing_reader_rejects_legacy_key_only_speed_rows() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-legacy-timing");
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context =
        crate::digest::bytes(format!("{candidate}:hot:verified-local:{changed}").as_bytes());
    let input = super::graph::surface_input_digest(
        super::surface_by_id("fmt_check").expect("fmt surface"),
        candidate,
        &changed,
        &context,
    );
    crate::json_boundary::write_json(
        &root.join(NODE_TIMING_REL),
        &json!({
            "nodes": [{
                "node_id": "fmt_check",
                "candidate_digest": candidate,
                "tier": "hot",
                "cache_mode": "verified-local",
                "input_digest": input,
                "canonical_full_command": "cargo fmt --all --check",
                "timing_status": "pass",
                "cache_honesty": "pass",
                "failure_class": "none",
                "baseline_duration_ms": 1000,
                "verified_local_duration_ms": 1
            }]
        }),
    )
    .expect("legacy timing artifact");

    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed,
        &context,
    );
    assert!(timings.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup legacy timing");
}

#[test]
fn node_timing_reader_rejects_launched_rows_without_baseline_exit_code() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("live-loop-missing-exit");
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context =
        crate::digest::bytes(format!("{candidate}:hot:verified-local:{changed}").as_bytes());
    let input = super::graph::surface_input_digest(
        super::surface_by_id("fmt_check").expect("fmt surface"),
        candidate,
        &changed,
        &context,
    );
    let mut row = current_timing_row(candidate, &input, "fail", "canonical_full_command_failed");
    row.as_object_mut()
        .expect("timing row object")
        .remove("baseline_exit_code");
    crate::json_boundary::write_json(&root.join(NODE_TIMING_REL), &json!({ "nodes": [row] }))
        .expect("timing artifact");

    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed,
        &context,
    );
    assert!(timings.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup missing-exit timing");
}

fn current_timing_row(
    candidate: &str,
    input: &str,
    timing_status: &str,
    failure_class: &str,
) -> serde_json::Value {
    let exit_code = if timing_status == "pass" { 0 } else { 101 };
    let stdout_digest = digest("stdout");
    let stderr_digest = digest("stderr");
    let output_digest = digest("output");
    let result_digest = digest("result");
    let mut row = json!({
        "node_id": "fmt_check",
        "candidate_digest": candidate,
        "tier": "hot",
        "cache_mode": "verified-local",
        "input_digest": input,
        "current_input_digest": input,
        "canonical_full_command": "cargo fmt --all --check",
        "timing_status": timing_status,
        "failure_class": failure_class,
        "baseline_exit_code": exit_code,
        "baseline_launch_error": false,
        "cache_honesty": "pass",
        "baseline_duration_ms": 1000,
        "verified_local_duration_ms": 2,
        "proof_kind": "executed",
        "cache_hit": false,
        "cache_key": digest("cache"),
        "validator_version": "ultragoal-rust",
        "law_version": "observability-live-loop",
        "schema_version": "harness-ultragoal.live-loop-node-timing.v1",
        "fixture_version": "source-tree-current",
        "work_unit_count": 1,
        "actual_work_duration_ms": 2,
        "graph_overhead_ms": 1,
        "equivalence_status": "executed_current_candidate_not_cache_replay",
        "invalidation_proof": "cache_not_used_current_command_executed",
        "telemetry_reconciliation_status": "pass",
        "verified_local_command": "cargo fmt --all --check",
        "verified_local_command_argv": ["bash", "-lc", "cargo fmt --all --check"],
        "verified_local_stdout_digest": stdout_digest,
        "verified_local_stderr_digest": stderr_digest,
        "verified_local_exit_code": exit_code,
        "output_digest": output_digest,
        "result_digest": result_digest,
        "verified_local_output_digest": output_digest,
        "verified_local_result_digest": result_digest,
        "where_failed": "loop.measure.fmt_check.canonical_full_command",
        "why_failed": "canonical full command exited nonzero while measuring live-loop node",
        "next_repair": "run cargo fmt and rerun loop measure",
        "affected_set_status": "clean_worktree_no_affected_files"
    });
    let object = row.as_object_mut().expect("timing row object");
    object.insert(
        "command_argv".to_string(),
        json!(["bash", "-lc", "cargo fmt --all --check"]),
    );
    object.insert("exit_status".to_string(), json!(exit_code));
    object.insert("telemetry_reconciliation_duration_ms".to_string(), json!(3));
    object.insert("reconciled_command_duration_ms".to_string(), json!(6));
    object.insert("product_latency_ms".to_string(), json!(6));
    object.insert("receipt_paths".to_string(), json!([NODE_TIMING_REL]));
    object.insert("artifact_paths".to_string(), json!([NODE_TIMING_REL]));
    row
}

fn digest(label: &str) -> String {
    crate::digest::bytes(label.as_bytes())
}
