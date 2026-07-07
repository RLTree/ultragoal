use super::{digest, insert};
use serde_json::json;

pub(crate) fn verified_cache_row(candidate: &str) -> serde_json::Value {
    let result = digest('c');
    let output = digest('d');
    let mut row = json!({
        "node_id": "fmt_check",
        "proof_kind": "verified_cache_hit",
        "candidate_digest": candidate,
        "tier": "hot",
        "cache_mode": "verified-local",
        "timing_status": "pass",
        "failure_class": "none",
        "where_failed": "none",
        "why_failed": "none",
        "next_repair": "none"
    });
    insert(&mut row, "cache_hit", json!(true));
    insert(&mut row, "work_unit_count", json!(0));
    insert(&mut row, "actual_work_duration_ms", json!(1));
    insert(&mut row, "graph_overhead_ms", json!(1));
    insert(&mut row, "result_digest", json!(result));
    insert(&mut row, "output_digest", json!(output));
    insert(&mut row, "telemetry_reconciliation_status", json!("pass"));
    insert(&mut row, "telemetry_reconciliation_duration_ms", json!(1));
    insert(&mut row, "reconciled_command_duration_ms", json!(3));
    insert(&mut row, "product_latency_ms", json!(2));
    insert(
        &mut row,
        "claim_name",
        json!("source-local speed node timing claim"),
    );
    insert(
        &mut row,
        "product_behavior_observed",
        json!("cargo fmt --all --check verified cache replay"),
    );
    insert(
        &mut row,
        "proof_surface",
        json!(
            "speed node receipt with cache key, current input digest, prior result digest, replayed output digest, and invalidation proof"
        ),
    );
    insert(
        &mut row,
        "independent_reconciliation_surface",
        json!("same-candidate telemetry and verified cache equivalence replay"),
    );
    insert(
        &mut row,
        "claim_impact",
        json!("supports_live_loop_node_timing_only_no_readiness_release_completion_update_goal"),
    );
    insert_command_fields(&mut row);
    insert_context_placeholders(&mut row);
    insert(&mut row, "prior_result_digest", json!(result));
    insert(&mut row, "replayed_output_digest", json!(output));
    insert(&mut row, "cache_equivalence_status", json!("pass"));
    insert(
        &mut row,
        "equivalence_status",
        json!("verified_same_candidate_cache_replay"),
    );
    insert(
        &mut row,
        "invalidation_proof",
        json!("cache_key_current_input_digest_command_versions_and_candidate_row_matched"),
    );
    row
}

fn insert_command_fields(row: &mut serde_json::Value) {
    insert(
        row,
        "verified_local_command_argv",
        json!(["bash", "-lc", "cargo fmt --all --check"]),
    );
    insert(
        row,
        "command_argv",
        json!(["bash", "-lc", "cargo fmt --all --check"]),
    );
    insert(row, "verified_local_exit_code", json!(0));
    insert(row, "exit_status", json!(0));
    insert(
        row,
        "receipt_paths",
        json!(["validation_artifacts/observability/live-loop-node-timing.json"]),
    );
    insert(
        row,
        "artifact_paths",
        json!(["validation_artifacts/observability/live-loop-node-timing.json"]),
    );
    insert(
        row,
        "receipt_path",
        json!("validation_artifacts/observability/live-loop-node-timing.json"),
    );
    insert(
        row,
        "verified_local_failure",
        json!({
            "receipt": "validation_artifacts/observability/live-loop/commands/fmt_check-command-observation.json"
        }),
    );
}

fn insert_context_placeholders(row: &mut serde_json::Value) {
    insert(row, "cache_key", json!(digest('e')));
    insert(row, "input_digest", json!(digest('f')));
    insert(row, "current_input_digest", json!(digest('f')));
    insert(row, "audit_context_digest", json!(digest('g')));
    insert(
        row,
        "validator_version",
        json!(crate::cli::live_loop::validator_version()),
    );
    insert(
        row,
        "law_version",
        json!(crate::cli::live_loop::law_version()),
    );
    insert(
        row,
        "schema_version",
        json!(crate::cli::live_loop::schema_version()),
    );
    insert(
        row,
        "fixture_version",
        json!(crate::cli::live_loop::fixture_version()),
    );
}
