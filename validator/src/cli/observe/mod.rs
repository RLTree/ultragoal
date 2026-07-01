use serde_json::Value;
use std::path::{Path, PathBuf};

pub(crate) mod explain;
pub(crate) mod query;
pub(crate) mod stack;
pub(crate) mod telemetry;
pub(crate) mod types;

use types::{ObserveCommand, ObserveOperation};

pub(crate) fn parse(raw: &[String]) -> Result<Option<ObserveCommand>, String> {
    if raw.first().map(String::as_str) != Some("observe") {
        return Ok(None);
    }
    let operation = operation(raw)?;
    Ok(Some(ObserveCommand {
        operation,
        receipt: opt_path(raw, "--receipt"),
        query: opt_string(raw, "--query"),
        run_id: opt_string(raw, "--run-id"),
        correlation_id: opt_string(raw, "--correlation-id"),
        claim_id: opt_string(raw, "--claim-id"),
        check_id: opt_string(raw, "--check-id"),
        law_id: opt_string(raw, "--law-id"),
        row_limit: opt_usize(raw, "--limit").unwrap_or(100),
        byte_limit: opt_usize(raw, "--byte-limit").unwrap_or(262_144),
        timeout_ms: opt_u64(raw, "--timeout-ms").unwrap_or(30_000),
    }))
}

pub(crate) fn run(root: &Path, command: &ObserveCommand) -> Result<i32, String> {
    let value = match command.operation {
        ObserveOperation::StackUp
        | ObserveOperation::StackHealth
        | ObserveOperation::StackSmoke
        | ObserveOperation::StackDown
        | ObserveOperation::StackGcPlan
        | ObserveOperation::StackGcDryRun
        | ObserveOperation::StackGcApply => stack::run(root, command)?,
        ObserveOperation::LogsQuery
        | ObserveOperation::MetricsQuery
        | ObserveOperation::TracesQuery => query::run(root, command)?,
        ObserveOperation::Snapshot | ObserveOperation::Prove => telemetry::prove(root, command)?,
        ObserveOperation::ExplainFailure
        | ObserveOperation::ExplainClaim
        | ObserveOperation::ExplainCheck
        | ObserveOperation::ExplainLaw => explain::run(root, command)?,
    };
    write_and_print(root, command, &value)
}

fn write_and_print(root: &Path, command: &ObserveCommand, value: &Value) -> Result<i32, String> {
    let receipt = command.receipt_rel();
    let absolute = if receipt.is_absolute() {
        receipt.clone()
    } else {
        root.join(&receipt)
    };
    crate::json_boundary::write_json(&absolute, value)?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("fail");
    println!(
        "ultragoal-observe {status} operation={} candidate={} receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        command.operation.id(),
        value
            .get("candidate_digest")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        receipt.display(),
        value
            .get("run_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("correlation_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("event")
            .and_then(|event| event.get("claim_impact"))
            .and_then(Value::as_str)
            .or_else(|| value.get("claim_impact").and_then(Value::as_str))
            .unwrap_or("<missing>"),
        csv(value.get("supported_claims")),
        csv(value.get("blocked_claims"))
    );
    if status != "pass" {
        let metric_query =
            crate::cli::observe::query::bounded_metric_query_for_operation(command.operation.id());
        println!(
            "failed_check={} why={} where={} claim_impact={} next_repair={} receipt={} run_id={} correlation_id={} query_logs='ultragoal observe logs query --run-id {} --limit 100' query_metrics='ultragoal observe metrics query --query '{}' --limit 100' query_traces='ultragoal observe traces query --run-id {} --limit 100'",
            value
                .get("check_id")
                .and_then(Value::as_str)
                .unwrap_or(types::CHECK_ID),
            value
                .get("why_failed")
                .and_then(Value::as_str)
                .unwrap_or("observability proof failed"),
            value
                .get("where_failed")
                .and_then(Value::as_str)
                .unwrap_or("observe command"),
            value
                .get("event")
                .and_then(|event| event.get("claim_impact"))
                .and_then(Value::as_str)
                .or_else(|| value.get("claim_impact").and_then(Value::as_str))
                .unwrap_or("readiness_release_completion_update_goal_blocked"),
            value
                .get("next_repair")
                .and_then(Value::as_str)
                .unwrap_or("run observe stack health and smoke"),
            receipt.display(),
            value
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            value
                .get("correlation_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            value
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            metric_query,
            value
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        );
    }
    Ok(i32::from(status != "pass"))
}

fn operation(raw: &[String]) -> Result<ObserveOperation, String> {
    match raw {
        [_, a, b, ..] if a == "stack" && b == "up" => Ok(ObserveOperation::StackUp),
        [_, a, b, ..] if a == "stack" && b == "health" => Ok(ObserveOperation::StackHealth),
        [_, a, b, ..] if a == "stack" && b == "smoke" => Ok(ObserveOperation::StackSmoke),
        [_, a, b, ..] if a == "stack" && b == "down" => Ok(ObserveOperation::StackDown),
        [_, a, b, c, ..] if a == "stack" && b == "gc" && c == "plan" => {
            Ok(ObserveOperation::StackGcPlan)
        }
        [_, a, b, c, ..] if a == "stack" && b == "gc" && c == "dry-run" => {
            Ok(ObserveOperation::StackGcDryRun)
        }
        [_, a, b, c, ..] if a == "stack" && b == "gc" && c == "apply" => {
            Ok(ObserveOperation::StackGcApply)
        }
        [_, a, b, ..] if a == "logs" && b == "query" => Ok(ObserveOperation::LogsQuery),
        [_, a, b, ..] if a == "metrics" && b == "query" => Ok(ObserveOperation::MetricsQuery),
        [_, a, b, ..] if a == "traces" && b == "query" => Ok(ObserveOperation::TracesQuery),
        [_, a, ..] if a == "snapshot" => Ok(ObserveOperation::Snapshot),
        [_, a, ..] if a == "prove" => Ok(ObserveOperation::Prove),
        [_, a, ..] if a == "explain-failure" => Ok(ObserveOperation::ExplainFailure),
        [_, a, ..] if a == "explain-claim" => Ok(ObserveOperation::ExplainClaim),
        [_, a, ..] if a == "explain-check" => Ok(ObserveOperation::ExplainCheck),
        [_, a, ..] if a == "explain-law" => Ok(ObserveOperation::ExplainLaw),
        _ => Err("unknown ultragoal observe command".to_string()),
    }
}

fn opt_string(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    opt_string(args, key).map(PathBuf::from)
}

fn opt_usize(args: &[String], key: &str) -> Option<usize> {
    opt_string(args, key).and_then(|value| value.parse().ok())
}

fn opt_u64(args: &[String], key: &str) -> Option<u64> {
    opt_string(args, key).and_then(|value| value.parse().ok())
}

fn csv(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_else(|| "none".to_string())
}

#[cfg(test)]
pub(crate) fn csv_for_test(value: &Value) -> String {
    csv(value.get("supported_claims"))
}
