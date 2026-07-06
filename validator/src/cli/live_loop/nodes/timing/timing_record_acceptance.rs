use super::{NODE_TIMING_REL, read_current};
use serde_json::{Value, json};

#[test]
fn node_timing_reader_accepts_verified_cache_hit_with_equivalence() {
    let timings = read_with_patch(json!({
        "proof_kind": "verified_cache_hit",
        "cache_hit": true,
        "work_unit_count": 0,
        "equivalence_status": "verified_same_candidate_cache_replay",
        "invalidation_proof": "cache_key_current_input_digest_command_versions_and_candidate_row_matched",
        "prior_result_digest": digest("result"),
        "replayed_output_digest": digest("output"),
        "cache_equivalence_status": "pass"
    }));

    assert_eq!(timings, 1);
}

#[test]
fn node_timing_reader_rejects_proof_shaped_rows_without_current_work_or_equivalence() {
    let cases = [
        json!({"actual_work_duration_ms": 3}),
        json!({"validator_version": ""}),
        json!({"verified_local_command": ""}),
        json!({"verified_local_command_argv": []}),
        json!({"result_digest": serde_json::Value::Null}),
        json!({"output_digest": serde_json::Value::Null}),
        json!({"result_digest": digest("different-result")}),
        json!({"output_digest": digest("different-output")}),
        json!({"verified_local_stdout_digest": "sha256:short"}),
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

fn read_with_patch(patch: Value) -> usize {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-timing-record-acceptance",
    );
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
    let mut row = current_timing_row(candidate, &input);
    let object = row.as_object_mut().expect("current timing row object");
    for (key, value) in patch.as_object().expect("patch object") {
        object.insert(key.clone(), value.clone());
    }
    crate::json_boundary::write_json(&root.join(NODE_TIMING_REL), &json!({"nodes": [row]}))
        .expect("timing artifact");
    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed,
        &context,
    );
    std::fs::remove_dir_all(root).expect("cleanup timing acceptance");
    timings.len()
}

fn current_timing_row(candidate: &str, input: &str) -> Value {
    let stdout_digest = digest("stdout");
    let stderr_digest = digest("stderr");
    let output_digest = digest("output");
    let result_digest = digest("result");
    json!({
        "node_id": "fmt_check",
        "candidate_digest": candidate,
        "tier": "hot",
        "cache_mode": "verified-local",
        "input_digest": input,
        "current_input_digest": input,
        "canonical_full_command": "cargo fmt --all --check",
        "timing_status": "pass",
        "failure_class": "none",
        "baseline_exit_code": 0,
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
        "output_digest": output_digest,
        "result_digest": result_digest,
        "verified_local_output_digest": output_digest,
        "verified_local_result_digest": result_digest,
        "where_failed": "none",
        "why_failed": "none",
        "next_repair": "none",
        "affected_set_status": "clean_worktree_no_affected_files"
    })
}

fn digest(label: &str) -> String {
    crate::digest::bytes(label.as_bytes())
}
