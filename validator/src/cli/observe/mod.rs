use std::path::{Path, PathBuf};

mod command_roundtrip;
pub(crate) mod explain;
pub(crate) mod query;
mod snapshot;
pub(crate) mod stack;
mod stdout;
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
        target_command: opt_string(raw, "--command"),
        target_family: opt_string(raw, "--family"),
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
        ObserveOperation::LogsQuery => query::run(root, command, query::QueryKind::Logs)?,
        ObserveOperation::MetricsQuery => query::run(root, command, query::QueryKind::Metrics)?,
        ObserveOperation::TracesQuery => query::run(root, command, query::QueryKind::Traces)?,
        ObserveOperation::Snapshot => snapshot::run(root, command)?,
        ObserveOperation::Prove => telemetry::prove(root, command)?,
        ObserveOperation::CommandRoundtrip => command_roundtrip::run(root, command)?,
        ObserveOperation::ExplainFailure
        | ObserveOperation::ExplainClaim
        | ObserveOperation::ExplainCheck
        | ObserveOperation::ExplainLaw => explain::run(root, command)?,
    };
    stdout::write_and_print(root, command, &value)
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
        [_, a, ..] if a == "command-roundtrip" || a == "fit" => {
            Ok(ObserveOperation::CommandRoundtrip)
        }
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

#[cfg(test)]
fn csv(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_else(|| "none".to_string())
}

#[cfg(test)]
pub(crate) fn csv_for_test(value: &serde_json::Value) -> String {
    csv(value.get("supported_claims"))
}
