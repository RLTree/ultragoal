use serde_json::Value;
use std::path::Path;

pub(super) fn print_receipt(path: &Path, value: &Value) {
    for line in stdout_lines(path, value) {
        println!("{line}");
    }
}

pub(super) fn stdout_lines(path: &Path, value: &Value) -> Vec<String> {
    let mut lines = vec![summary_line(path, value)];
    if super::status(value) != "pass" {
        lines.push(failure_line(path, value));
    }
    lines
}

fn summary_line(path: &Path, value: &Value) -> String {
    format!(
        "ultragoal-final-packet {} proven={} operation=final-packet.prove candidate={} receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        super::status(value),
        proven(value),
        str_field(value, "candidate_digest", "<missing>"),
        path.display(),
        str_field(value, "run_id", "<missing>"),
        str_field(value, "correlation_id", "<missing>"),
        str_field(value, "claim_impact", "<missing>"),
        csv(&value["observability"]["supported_claims"]),
        csv(&value["observability"]["blocked_claims"])
    )
}

fn failure_line(path: &Path, value: &Value) -> String {
    let run_id = str_field(value, "run_id", "unknown");
    let metric_query =
        crate::cli::observe::query::bounded_metric_query_for_operation("final-packet.prove");
    format!(
        "failed_law={} failed_check={} why={} where={} claim_impact={} next_repair={} receipt={} run_id={} correlation_id={} query_logs='ultragoal observe logs query --run-id {} --limit 100' query_metrics='ultragoal observe metrics query --query '{}' --limit 100' query_traces='ultragoal observe traces query --run-id {} --limit 100'",
        str_field(value, "proof_law_id", "final-packet-proof"),
        str_field(value, "proof_check_id", "final-packet-proof"),
        str_field(value, "why_failed", "final packet proof failed"),
        str_field(value, "where_failed", "final-packet.prove"),
        str_field(
            value,
            "claim_impact",
            "readiness_release_completion_update_goal_blocked"
        ),
        str_field(
            value,
            "next_repair",
            "inspect final-packet proof references"
        ),
        path.display(),
        run_id,
        str_field(value, "correlation_id", "unknown"),
        run_id,
        metric_query,
        run_id
    )
}

fn proven(value: &Value) -> &'static str {
    if super::status(value) == "pass" {
        "final_packet_evidence_dereferenced"
    } else {
        "none"
    }
}

fn str_field<'a>(value: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or(fallback)
}

fn csv(value: &Value) -> String {
    let items = super::string_array(value);
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(",")
    }
}
