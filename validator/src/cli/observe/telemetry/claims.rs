use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::{Value, json};

pub(super) fn claim_ceiling(operation: ObserveOperation, status: &str) -> &'static str {
    if status == "pass" && matches!(operation, ObserveOperation::Prove) {
        "observability_stack_proven_only"
    } else if status == "pass" {
        "observability_observation_only"
    } else {
        "observability_gate_failed_completion_readiness_release_update_goal_blocked"
    }
}

pub(super) fn blocked(operation: ObserveOperation, status: &str) -> Value {
    if status == "pass" && !matches!(operation, ObserveOperation::Prove) {
        json!([
            "completion",
            "readiness",
            "release",
            "update_goal_eligibility"
        ])
    } else if status == "pass" {
        json!([
            "final_packet_correctness",
            "review_readiness",
            "package_readiness",
            "release_readiness",
            "completion",
            "update_goal_eligibility",
            "registry_exposure",
            "reviewer_exposure"
        ])
    } else {
        json!([
            "final_packet_correctness",
            "review_readiness",
            "package_readiness",
            "release_readiness",
            "completion",
            "update_goal_eligibility"
        ])
    }
}

pub(super) fn supported(operation: ObserveOperation, status: &str) -> Value {
    if status == "pass" && matches!(operation, ObserveOperation::Prove) {
        json!(["source_local_observability_stack"])
    } else if status == "pass" {
        json!(["observability_command_observation"])
    } else {
        json!([])
    }
}

pub(super) fn next_repair_for(
    operation: ObserveOperation,
    status: &str,
    failure: Option<&str>,
) -> &'static str {
    if status == "pass" {
        "keep receipt same-candidate and rerun source audit before any readiness claim"
    } else if failure.is_some_and(|text| {
        text.contains("first_failure=observability_command_telemetry_query_not_current")
            || text.contains("first_failure=observability_surface_telemetry_query_not_current")
    }) {
        "refresh the first same-candidate query proof named in why_failed: rerun the target command, query logs metrics traces for that run, then rerun observe prove"
    } else if failure.is_some_and(|text| {
        text.contains("first_failure=observability_command_telemetry_receipt_not_current")
            || text.contains("first_failure=observability_surface_telemetry_receipt_not_current")
            || text.contains("first_failure=observability_command_telemetry_receipt_missing")
            || text.contains("first_failure=observability_surface_telemetry_receipt_missing")
    }) {
        "refresh the first command receipt named in why_failed on the current candidate, then query logs metrics traces and rerun observe prove"
    } else if failure.is_some_and(|text| text.contains("command inventory")) {
        "repair the first_failure and control_board_first_incomplete named in why_failed, then rerun observe prove"
    } else if failure.is_some_and(|text| text.contains("requested telemetry target unavailable")) {
        "run target command once on the current candidate, query logs metrics traces, then rerun explain"
    } else if failure.is_some_and(|text| text.contains("observed telemetry candidate digest")) {
        "rerun target command on the current candidate before claiming observability command telemetry"
    } else if failure.is_some_and(|text| text.contains("observed telemetry failure is opaque")) {
        "repair target command stdout and telemetry fields: failure_class why_failed where_failed next_repair"
    } else {
        match operation {
            ObserveOperation::StackHealth => "run ultragoal observe stack up, then stack health",
            ObserveOperation::StackSmoke => {
                "start stack and rerun smoke until logs, metrics, and traces query"
            }
            ObserveOperation::LogsQuery
            | ObserveOperation::MetricsQuery
            | ObserveOperation::TracesQuery => {
                "run stack health and smoke, then rerun bounded query"
            }
            _ => "run observe stack health and observe stack smoke",
        }
    }
}

pub(super) fn bounds_status(command: &ObserveCommand) -> &'static str {
    if command.row_limit <= 1000 && command.byte_limit <= 1_048_576 && command.timeout_ms <= 30_000
    {
        "pass"
    } else {
        "fail"
    }
}
