use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

fn digest(ch: char) -> String {
    crate::self_tests::boundaries::workspace_fixtures::sha(ch)
}

fn temp_root() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let sequence = NEXT_TEMP_ROOT.fetch_add(1, Ordering::SeqCst);
    std::env::current_dir()
        .expect("cwd")
        .join("target")
        .join(format!(
            "ultragoal-performance-speed-nodes-{}-{stamp}-{sequence}",
            std::process::id()
        ))
}

#[test]
fn projects_current_verified_cache_timing_into_speed_proof() {
    let root = temp_root();
    std::fs::create_dir_all(root.join("validation_artifacts/observability")).expect("dir");
    let candidate = digest('a');
    let row = verified_cache_row(&candidate);
    let proof_kind = super::speed_proof_kind(&row).expect("proof kind");
    assert!(super::row_supports_speed_claim(&row, proof_kind));
    write_timing_fixture(&root, json!({"nodes": [row]}));
    crate::json_boundary::read_json(&root.join(super::NODE_TIMING_REL)).expect("read timing");

    let proof = super::speed_proof_value(&root, &candidate, true);
    assert_eq!(proof["status"], "pass");
    let node = &proof["nodes"][0];
    assert_eq!(node["node_id"], "fmt_check");
    assert_eq!(node["proof_kind"], "verified_cache_hit");
    assert_eq!(node["work_unit_count"], 0);
    assert_eq!(
        node["equivalence_status"],
        "verified_same_candidate_cache_replay"
    );
    assert_eq!(node["result_digest"], node["prior_result_digest"]);
    assert_eq!(node["output_digest"], node["replayed_output_digest"]);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn projects_current_executed_timing_into_speed_proof() {
    let root = temp_root();
    std::fs::create_dir_all(root.join("validation_artifacts/observability")).expect("dir");
    let candidate = digest('a');
    let row = executed_row(&candidate);
    let proof_kind = super::speed_proof_kind(&row).expect("proof kind");
    assert!(super::row_supports_speed_claim(&row, proof_kind));
    write_timing_fixture(&root, json!({"nodes": [row]}));

    let proof = super::speed_proof_value(&root, &candidate, true);
    assert_eq!(proof["status"], "pass");
    let node = &proof["nodes"][0];
    assert_eq!(node["node_id"], "fmt_check");
    assert_eq!(node["proof_kind"], "executed");
    assert_eq!(node["cache_hit"], false);
    assert_eq!(node["work_unit_count"], 3);
    assert_eq!(
        node["claim_impact"],
        "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"
    );
    assert!(node.get("cache_key").is_none());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn projects_speed_nodes_in_deterministic_order() {
    let root = temp_root();
    std::fs::create_dir_all(root.join("validation_artifacts/observability")).expect("dir");
    let candidate = digest('a');
    let mut later = verified_cache_row(&candidate);
    later["node_id"] = json!("z_later_check");
    let mut earlier = executed_row(&candidate);
    earlier["node_id"] = json!("a_earlier_check");
    write_timing_fixture(&root, json!({"nodes": [later, earlier]}));

    let proof = super::speed_proof_value(&root, &candidate, true);
    let nodes = proof["nodes"].as_array().expect("nodes");
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0]["node_id"], "a_earlier_check");
    assert_eq!(nodes[1]["node_id"], "z_later_check");
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn stale_failed_or_unknown_timing_rows_do_not_become_speed_proof() {
    let root = temp_root();
    std::fs::create_dir_all(root.join("validation_artifacts/observability")).expect("dir");
    let candidate = digest('a');
    let mut stale = verified_cache_row(&digest('b'));
    stale["node_id"] = json!("stale");
    let mut failed = verified_cache_row(&candidate);
    failed["node_id"] = json!("failed");
    failed["timing_status"] = json!("fail");
    let mut unknown = verified_cache_row(&candidate);
    unknown["node_id"] = json!("unknown");
    unknown["proof_kind"] = json!("planned");
    let mut missing_kind = verified_cache_row(&candidate);
    missing_kind["node_id"] = json!("missing-proof-kind");
    missing_kind
        .as_object_mut()
        .expect("row object")
        .remove("proof_kind");
    write_timing_fixture(
        &root,
        json!({"nodes": [stale, failed, unknown, missing_kind]}),
    );

    let proof = super::speed_proof_value(&root, &candidate, true);
    assert_eq!(proof["status"], "blocked");
    assert_eq!(proof["nodes"].as_array().expect("nodes").len(), 0);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn proof_shaped_speed_rows_without_equivalence_do_not_become_speed_proof() {
    let root = temp_root();
    std::fs::create_dir_all(root.join("validation_artifacts/observability")).expect("dir");
    let candidate = digest('a');
    let mut zero_work = verified_cache_row(&candidate);
    zero_work["node_id"] = json!("zero-work");
    zero_work["proof_kind"] = json!("executed");
    zero_work["cache_hit"] = json!(false);
    let mut mismatched_cache = verified_cache_row(&candidate);
    mismatched_cache["node_id"] = json!("mismatched-cache");
    mismatched_cache["prior_result_digest"] = json!(digest('g'));
    write_timing_fixture(&root, json!({"nodes": [zero_work, mismatched_cache]}));

    let proof = super::speed_proof_value(&root, &candidate, true);
    assert_eq!(proof["status"], "blocked");
    assert_eq!(proof["nodes"].as_array().expect("nodes").len(), 0);
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn verified_cache_row(candidate: &str) -> serde_json::Value {
    let result = digest('c');
    let output = digest('d');
    json!({
        "node_id": "fmt_check",
        "proof_kind": "verified_cache_hit",
        "candidate_digest": candidate,
        "timing_status": "pass",
        "failure_class": "none",
        "cache_hit": true,
        "work_unit_count": 0,
        "actual_work_duration_ms": 1,
        "graph_overhead_ms": 1,
        "result_digest": result,
        "output_digest": output,
        "telemetry_reconciliation_status": "pass",
        "claim_impact": "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal",
        "verified_local_command_argv": ["bash", "-lc", "cargo fmt --all --check"],
        "verified_local_exit_code": 0,
        "receipt_path": "validation_artifacts/observability/live-loop-node-timing.json",
        "verified_local_failure": {
            "receipt": "validation_artifacts/observability/live-loop/commands/fmt_check-command-observation.json"
        },
        "cache_key": digest('e'),
        "current_input_digest": digest('f'),
        "validator_version": "ultragoal-rust",
        "law_version": "observability-live-loop",
        "schema_version": "harness-ultragoal.live-loop-node-timing.v1",
        "fixture_version": "source-tree-current",
        "prior_result_digest": result,
        "replayed_output_digest": output,
        "cache_equivalence_status": "pass",
        "equivalence_status": "verified_same_candidate_cache_replay",
        "invalidation_proof": "cache_key_current_input_digest_command_versions_and_candidate_row_matched"
    })
}

fn executed_row(candidate: &str) -> serde_json::Value {
    json!({
        "node_id": "fmt_check",
        "proof_kind": "executed",
        "candidate_digest": candidate,
        "timing_status": "pass",
        "failure_class": "none",
        "cache_hit": false,
        "work_unit_count": 3,
        "actual_work_duration_ms": 1253,
        "graph_overhead_ms": 1,
        "result_digest": digest('c'),
        "output_digest": digest('d'),
        "telemetry_reconciliation_status": "pass",
        "claim_impact": "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal",
        "verified_local_command_argv": ["bash", "-lc", "cargo fmt --all --check"],
        "verified_local_exit_code": 0,
        "receipt_path": "validation_artifacts/observability/live-loop-node-timing.json",
        "verified_local_failure": {
            "receipt": "validation_artifacts/observability/live-loop/commands/fmt_check-command-observation.json"
        }
    })
}

fn write_timing_fixture(root: &std::path::Path, value: serde_json::Value) {
    std::fs::write(
        root.join(super::NODE_TIMING_REL),
        serde_json::to_string_pretty(&value).expect("json"),
    )
    .expect("timing");
}
