use super::*;
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
fn telemetry_stdout_fields_fall_back_to_cached_reconciliation() {
    let row = json!({
        "telemetry_reconciliation": {
            "status": "pass",
            "reconciliation_mode": "verified_same_candidate_telemetry_reuse",
            "cached_reconciliation": {
                "run_id": "run-cached",
                "correlation_id": "corr-cached",
                "trace_id": "trace-cached",
                "command_observation_receipt": "validation_artifacts/observability/live-loop/commands/fmt.json",
                "logs_query": {"value": {"query": "logs query"}},
                "metrics_query": {"value": {"query": "metrics query"}},
                "traces_query": {"value": {"query": "traces query"}},
                "first_failed_roundtrip": {"roundtrip": "metrics"}
            }
        }
    });

    assert_eq!(telemetry_text(&row, "run_id"), Some("run-cached"));
    assert_eq!(telemetry_text(&row, "correlation_id"), Some("corr-cached"));
    assert_eq!(telemetry_text(&row, "trace_id"), Some("trace-cached"));
    assert_eq!(telemetry_query(&row, "logs_query"), Some("logs query"));
    assert_eq!(
        telemetry_json(&row, "first_failed_roundtrip"),
        "{\"roundtrip\":\"metrics\"}"
    );
}

#[test]
fn telemetry_stdout_fields_read_direct_query_receipt_shape() {
    let row = json!({
        "telemetry_reconciliation": {
            "status": "pass",
            "run_id": "run-direct",
            "logs_query": {"query": "logs direct query"},
            "metrics_query": {"query": "metrics direct query"},
            "traces_query": {"query": "traces direct query"}
        }
    });

    assert_eq!(telemetry_text(&row, "run_id"), Some("run-direct"));
    assert_eq!(
        telemetry_query(&row, "logs_query"),
        Some("logs direct query")
    );
    assert_eq!(
        telemetry_query(&row, "metrics_query"),
        Some("metrics direct query")
    );
    assert_eq!(
        telemetry_query(&row, "traces_query"),
        Some("traces direct query")
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
