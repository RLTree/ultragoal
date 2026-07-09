use super::super::replayable_cache_records;
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;

#[path = "replay_rows.rs"]
mod replay_rows;
use self::replay_rows::{
    COMMAND_OBSERVATION_REL, WithValue, output_digest, result_digest, row,
    write_command_observation,
};

#[test]
fn telemetry_partial_latest_measurement_without_external_authority_is_not_replayable() {
    let root = temp_root("live-loop-cache-records-telemetry-partial");
    let candidate = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let prior = row(candidate, "pass", "none");
    let latest = row(
        candidate,
        "partial",
        "live_loop_telemetry_reconciliation_missing",
    )
    .with_value("telemetry_reconciliation_status", json!("query_failed"))
    .with_value(
        "telemetry_reconciliation",
        json!({
            "status": "query_failed",
            "command_observation_receipt": COMMAND_OBSERVATION_REL
        }),
    )
    .with_value("observability_status", json!("partial"))
    .with_value("speed_claim_status", json!("withheld"))
    .with_value(
        "observability_failure_class",
        json!("live_loop_observability_partial"),
    );
    write_command_observation(&root, &latest);

    let records = replayable_cache_records(
        &root,
        &json!({"nodes": [prior]}),
        &[latest],
        "hot",
        "verified-local",
    );

    assert!(records.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup cache records");
}

#[test]
fn existing_local_only_cache_records_are_not_available_when_latest_node_is_unusable() {
    let root = temp_root("live-loop-cache-records-prior");
    let candidate = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let prior = row(candidate, "pass", "none");
    let latest = row(candidate, "fail", "canonical_full_command_failed")
        .with_value("validation_status", json!("fail"))
        .with_value("validation_cache_status", json!("not_reusable"));
    write_command_observation(&root, &prior);

    let records = replayable_cache_records(
        &root,
        &json!({"cache_records": [prior]}),
        &[latest],
        "hot",
        "verified-local",
    );

    assert!(records.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup cache records");
}

#[test]
fn compact_cache_records_drop_cached_reconciliation_without_external_authority() {
    let root = temp_root("live-loop-cache-records-reconciliation");
    let candidate = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let row = row(candidate, "pass", "none").with_value(
        "telemetry_reconciliation",
        json!({
            "status": "pass",
            "command_observation_receipt": COMMAND_OBSERVATION_REL,
            "reconciliation_mode": "verified_same_candidate_telemetry_reuse",
            "cached_reconciliation": {
                "run_id": "run-cached",
                "logs_query": {"value": {"query": "logs query"}},
                "explain_failure": {"value": {"root_cause": "cached root cause"}}
            }
        }),
    );
    write_command_observation(&root, &row);

    let records = replayable_cache_records(&root, &json!({}), &[row], "hot", "verified-local");

    assert!(records.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup cache records");
}

#[test]
fn compact_cache_records_drop_bad_proof_shaped_rows() {
    let root = temp_root("live-loop-cache-records-bad-proof");
    let candidate = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let real_row = row(candidate, "pass", "none");
    write_command_observation(&root, &real_row);
    let forged_stdout_digest = crate::digest::bytes(b"forged-stdout");
    let forged_stderr_digest = crate::digest::bytes(b"forged-stderr");
    let forged_output_digest = crate::digest::bytes(
        format!("stdout={forged_stdout_digest};stderr={forged_stderr_digest}").as_bytes(),
    );
    let forged_result_digest = crate::digest::bytes(
        format!("exit=0;launch=false;output={forged_output_digest}").as_bytes(),
    );
    let forged_digest_row = row(candidate, "pass", "none")
        .with_value("verified_local_stdout_digest", json!(forged_stdout_digest))
        .with_value("verified_local_stderr_digest", json!(forged_stderr_digest))
        .with_value("output_digest", json!(forged_output_digest))
        .with_value("verified_local_output_digest", json!(forged_output_digest))
        .with_value("result_digest", json!(forged_result_digest))
        .with_value("verified_local_result_digest", json!(forged_result_digest));
    let zero_test_row = row(candidate, "pass", "none")
        .with_value("node_id", json!("live_loop_measurement_rust_tests"))
        .with_value("verified_local_executed_test_count", json!(0));

    let records = replayable_cache_records(
        &root,
        &json!({}),
        &[forged_digest_row, zero_test_row],
        "hot",
        "verified-local",
    );

    assert!(records.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup cache records");
}

#[test]
fn compact_cache_records_drop_bad_verified_cache_hit_semantics() {
    let root = temp_root("live-loop-cache-records-bad-cache-hit");
    let candidate = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let row = row(candidate, "pass", "none")
        .with_value("proof_kind", json!("verified_cache_hit"))
        .with_value("cache_hit", json!(true))
        .with_value("work_unit_count", json!(1))
        .with_value("cache_equivalence_status", json!("pass"))
        .with_value("prior_result_digest", json!(result_digest()))
        .with_value("replayed_output_digest", json!(output_digest()));
    write_command_observation(&root, &row);

    let records = replayable_cache_records(&root, &json!({}), &[row], "hot", "verified-local");

    assert!(records.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup cache records");
}
