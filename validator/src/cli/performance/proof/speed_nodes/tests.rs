#[path = "evidence_requirement_tests.rs"]
mod evidence_requirement_tests;
#[path = "test_rows.rs"]
mod test_rows;
use serde_json::json;
use test_rows::{digest, executed_row, temp_root, verified_cache_row, write_timing_fixture};

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
    assert_eq!(node["claim_name"], "source-local speed node timing claim");
    assert_eq!(
        node["product_behavior_observed"],
        "cargo fmt --all --check verified cache replay"
    );
    assert_eq!(
        node["proof_surface"],
        "speed node receipt with cache key, current input digest, prior result digest, replayed output digest, and invalidation proof"
    );
    assert_eq!(
        node["independent_reconciliation_surface"],
        "same-candidate telemetry and verified cache equivalence replay"
    );
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
    assert_eq!(node["claim_name"], "source-local speed node timing claim");
    assert_eq!(
        node["product_behavior_observed"],
        "cargo fmt --all --check command execution"
    );
    assert_eq!(
        node["proof_surface"],
        "speed node receipt with command argv, exit status, result digest, output digest, and telemetry reconciliation"
    );
    assert_eq!(
        node["independent_reconciliation_surface"],
        "same-candidate telemetry and performance receipt"
    );
    assert_eq!(
        node["claim_impact"],
        "supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"
    );
    assert!(node.get("cache_key").is_none());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn speed_claim_accepts_artifact_path_when_receipt_path_is_absent() {
    let candidate = digest('a');
    let mut row = executed_row(&candidate);
    row.as_object_mut()
        .expect("speed row object")
        .remove("receipt_paths");
    let proof_kind = super::speed_proof_kind(&row).expect("proof kind");

    assert!(super::row_supports_speed_claim(&row, proof_kind));
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
fn stale_rows_are_ignored_but_current_blockers_stay_visible() {
    let root = temp_root();
    std::fs::create_dir_all(root.join("validation_artifacts/observability")).expect("dir");
    let candidate = digest('a');
    let mut stale = verified_cache_row(&digest('b'));
    stale["node_id"] = json!("stale");
    let mut failed = verified_cache_row(&candidate);
    failed["node_id"] = json!("failed");
    failed["timing_status"] = json!("fail");
    failed["failure_class"] = json!("live_loop_speedup_target_missed");
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
    let nodes = proof["nodes"].as_array().expect("nodes");
    assert_eq!(nodes.len(), 3);
    assert_eq!(proof["first_blocker"]["node_id"], "failed");
    assert_eq!(
        proof["first_blocker"]["failure_class"],
        "live_loop_speedup_target_missed"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn unknown_speed_proof_kind_blocks_claim_without_neighbor_failure() {
    let root = temp_root();
    std::fs::create_dir_all(root.join("validation_artifacts/observability")).expect("dir");
    let candidate = digest('a');
    let mut unknown = verified_cache_row(&candidate);
    unknown["node_id"] = json!("unknown-proof-kind");
    unknown["proof_kind"] = json!("planned");
    assert!(super::speed_proof_kind(&unknown).is_none());
    write_timing_fixture(&root, json!({"nodes": [unknown]}));

    let proof = super::speed_proof_value(&root, &candidate, true);
    assert_eq!(proof["status"], "blocked");
    assert_eq!(proof["first_blocker"]["node_id"], "unknown-proof-kind");
    assert_eq!(proof["first_blocker"]["failure_class"], "none");
    std::fs::remove_dir_all(root).expect("cleanup unknown proof kind");
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
    let nodes = proof["nodes"].as_array().expect("nodes");
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0]["node_id"], "mismatched-cache");
    assert_eq!(nodes[1]["node_id"], "zero-work");
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn speed_rows_without_reconciled_product_latency_are_rejected() {
    let root = temp_root();
    std::fs::create_dir_all(root.join("validation_artifacts/observability")).expect("dir");
    let candidate = digest('a');
    let mut mismatched_latency = executed_row(&candidate);
    mismatched_latency["node_id"] = json!("mismatched-product-latency");
    mismatched_latency["reconciled_command_duration_ms"] = json!(1);
    let mut rows = vec![mismatched_latency];
    for field in [
        "actual_work_duration_ms",
        "graph_overhead_ms",
        "telemetry_reconciliation_duration_ms",
        "reconciled_command_duration_ms",
        "product_latency_ms",
    ] {
        let mut row = executed_row(&candidate);
        row["node_id"] = json!(format!("missing-{field}"));
        row.as_object_mut().expect("row object").remove(field);
        rows.push(row);
    }
    write_timing_fixture(&root, json!({"nodes": rows}));

    let proof = super::speed_proof_value(&root, &candidate, true);
    assert_eq!(proof["status"], "blocked");
    let nodes = proof["nodes"].as_array().expect("nodes");
    assert_eq!(nodes.len(), 6);
    assert!(
        nodes
            .iter()
            .all(super::claim_readiness::node_blocks_speed_claim)
    );
    std::fs::remove_dir_all(root).expect("cleanup product latency");
}
