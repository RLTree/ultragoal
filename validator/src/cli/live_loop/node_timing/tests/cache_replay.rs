use super::super::replayable_cache_records;
use serde_json::json;

#[test]
fn telemetry_partial_latest_measurement_remains_replayable_for_validation() {
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
        json!({"status": "query_failed"}),
    )
    .with_value("observability_status", json!("partial"))
    .with_value("speed_claim_status", json!("withheld"))
    .with_value(
        "observability_failure_class",
        json!("live_loop_observability_partial"),
    );

    let records = replayable_cache_records(
        &json!({"nodes": [prior]}),
        &[latest],
        "hot",
        "verified-local",
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["timing_status"], "partial");
    assert_eq!(records[0]["validation_status"], "pass");
    assert_eq!(records[0]["validation_cache_status"], "reusable");
    assert_eq!(
        records[0]["telemetry_reconciliation_status"],
        "query_failed"
    );
}

#[test]
fn existing_cache_records_remain_available_when_latest_node_is_unusable() {
    let candidate = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let prior = row(candidate, "pass", "none");
    let latest = row(candidate, "fail", "canonical_full_command_failed")
        .with_value("validation_status", json!("fail"))
        .with_value("validation_cache_status", json!("not_reusable"));

    let records = replayable_cache_records(
        &json!({"cache_records": [prior]}),
        &[latest],
        "hot",
        "verified-local",
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["validation_status"], "pass");
    assert_eq!(records[0]["timing_status"], "pass");
}

#[test]
fn compact_cache_records_preserve_cached_reconciliation_evidence() {
    let candidate = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let row = row(candidate, "pass", "none").with_value(
        "telemetry_reconciliation",
        json!({
            "status": "pass",
            "reconciliation_mode": "verified_same_candidate_telemetry_reuse",
            "cached_reconciliation": {
                "run_id": "run-cached",
                "logs_query": {"value": {"query": "logs query"}},
                "explain_failure": {"value": {"root_cause": "cached root cause"}}
            }
        }),
    );

    let records = replayable_cache_records(&json!({}), &[row], "hot", "verified-local");

    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0]["telemetry_reconciliation"]["cached_reconciliation"]["run_id"],
        "run-cached"
    );
    assert_eq!(
        records[0]["telemetry_reconciliation"]["cached_reconciliation"]["logs_query"]["value"]["query"],
        "logs query"
    );
}

fn row(candidate: &str, timing_status: &str, failure_class: &str) -> serde_json::Value {
    let digest = crate::digest::bytes;
    let output_digest = digest(b"output");
    let result_digest = digest(b"result");
    json!({
        "node_id": "fmt_check",
        "candidate_digest": candidate,
        "tier": "hot",
        "cache_mode": "verified-local",
        "input_digest": digest(b"input"),
        "current_input_digest": digest(b"input"),
        "canonical_full_command": "cargo fmt --all --check",
        "timing_status": timing_status,
        "failure_class": failure_class,
        "validation_status": "pass",
        "validation_cache_status": "reusable",
        "observability_status": "pass",
        "speed_claim_status": "supported",
        "observability_failure_class": "none",
        "proof_kind": "executed",
        "cache_hit": false,
        "cache_key": digest(b"cache"),
        "cache_honesty": "pass",
        "telemetry_reconciliation_status": "pass",
        "telemetry_reconciliation": {"status": "pass"},
        "exit_status": 0,
        "verified_local_launch_error": false,
        "validator_version": crate::cli::live_loop::graph::validator_version(),
        "law_version": crate::cli::live_loop::graph::law_version(),
        "schema_version": crate::cli::live_loop::graph::schema_version(),
        "fixture_version": crate::cli::live_loop::graph::fixture_version(),
        "runtime_execution_model": crate::cli::live_loop::graph::runtime_execution_model(),
        "result_digest": result_digest,
        "output_digest": output_digest,
        "verified_local_result_digest": result_digest,
        "verified_local_output_digest": output_digest,
        "verified_local_stdout_digest": digest(b"stdout"),
        "verified_local_stderr_digest": digest(b"stderr"),
        "baseline_stdout_digest": digest(b"baseline-stdout"),
        "baseline_stderr_digest": digest(b"baseline-stderr")
    })
}

trait WithValue {
    fn with_value(self, key: &str, value: serde_json::Value) -> Self;
}

impl WithValue for serde_json::Value {
    fn with_value(mut self, key: &str, value: serde_json::Value) -> Self {
        self.as_object_mut()
            .expect("timing row object")
            .insert(key.to_string(), value);
        self
    }
}
