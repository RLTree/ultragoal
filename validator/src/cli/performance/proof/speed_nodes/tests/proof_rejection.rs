use super::test_rows::{
    bind_current_context, digest, executed_row, temp_root, verified_cache_row, write_timing_fixture,
};
use serde_json::json;

#[test]
fn proof_shaped_speed_rows_without_equivalence_do_not_become_speed_proof() {
    let root = temp_root();
    std::fs::create_dir_all(root.join("validation_artifacts/observability")).expect("dir");
    let candidate = digest('a');
    let mut zero_work = bind_current_context(&root, verified_cache_row(&candidate));
    zero_work["proof_kind"] = json!("executed");
    zero_work["cache_hit"] = json!(false);
    let mut mismatched_cache = bind_current_context(&root, verified_cache_row(&candidate));
    mismatched_cache["node_id"] = json!("namespace_check");
    mismatched_cache = bind_current_context(&root, mismatched_cache);
    mismatched_cache["prior_result_digest"] = json!(digest('g'));
    write_timing_fixture(&root, json!({"nodes": [zero_work, mismatched_cache]}));

    let proof = super::super::speed_proof_value(&root, &candidate, true);
    assert_eq!(proof["status"], "blocked");
    let nodes = proof["nodes"].as_array().expect("nodes");
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0]["node_id"], "fmt_check");
    assert_eq!(nodes[1]["node_id"], "namespace_check");
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn speed_rows_without_reconciled_product_latency_are_rejected() {
    let root = temp_root();
    std::fs::create_dir_all(root.join("validation_artifacts/observability")).expect("dir");
    let candidate = digest('a');
    let mut rows = vec![mismatched_latency_row(&root, &candidate)];
    rows.extend(missing_latency_rows(&root, &candidate));
    write_timing_fixture(&root, json!({"nodes": rows}));

    let proof = super::super::speed_proof_value(&root, &candidate, true);
    assert_eq!(proof["status"], "blocked");
    let nodes = proof["nodes"].as_array().expect("nodes");
    assert_eq!(nodes.len(), 6);
    assert!(
        nodes
            .iter()
            .all(super::super::claim_readiness::node_blocks_speed_claim)
    );
    std::fs::remove_dir_all(root).expect("cleanup product latency");
}

#[test]
fn speed_proof_ignores_rows_with_stale_currentness_context() {
    let root = temp_root();
    std::fs::create_dir_all(root.join("validation_artifacts/observability")).expect("dir");
    let candidate = digest('a');
    let mut stale = bind_current_context(&root, executed_row(&candidate));
    stale["validator_version"] = json!(digest('z'));
    let current = bind_current_context(&root, verified_cache_row(&candidate));
    write_timing_fixture(&root, json!({"nodes": [stale, current]}));

    let proof = super::super::speed_proof_value(&root, &candidate, true);
    assert_eq!(proof["status"], "pass");
    let nodes = proof["nodes"].as_array().expect("nodes");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["proof_kind"], "verified_cache_hit");
    assert_eq!(
        nodes[0]["validator_version"],
        crate::cli::live_loop::validator_version()
    );
    std::fs::remove_dir_all(root).expect("cleanup stale currentness");
}

fn mismatched_latency_row(root: &std::path::Path, candidate: &str) -> serde_json::Value {
    let mut row = bind_current_context(root, executed_row(candidate));
    row["reconciled_command_duration_ms"] = json!(1);
    row
}

fn missing_latency_rows(root: &std::path::Path, candidate: &str) -> Vec<serde_json::Value> {
    [
        ("namespace_check", "actual_work_duration_ms"),
        ("line_caps_check", "graph_overhead_ms"),
        ("schema_validation", "telemetry_reconciliation_duration_ms"),
        ("package_inventory", "reconciled_command_duration_ms"),
        ("build_check", "product_latency_ms"),
    ]
    .into_iter()
    .map(|(node_id, field)| {
        let mut row = executed_row(candidate);
        row["node_id"] = json!(node_id);
        row = bind_current_context(root, row);
        row.as_object_mut().expect("row object").remove(field);
        row
    })
    .collect()
}
