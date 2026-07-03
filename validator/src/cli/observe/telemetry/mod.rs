use crate::cli::observe::types::ObserveCommand;
use serde_json::Value;
use std::path::Path;

mod claims;
mod command;
mod exporter;
mod identity;
mod metric;
mod query;
mod receipt;
mod record;
mod runtime;
mod spool;
#[cfg(test)]
mod tests;
mod trace;

pub(crate) fn base_receipt(
    root: &Path,
    command: &ObserveCommand,
    status: &str,
    failure: Option<&str>,
) -> Result<Value, String> {
    receipt::base(root, command, status, failure)
}

pub(crate) fn prove(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    if !live_stack_receipts_current(root, &candidate) {
        return base_receipt(
            root,
            command,
            "fail",
            Some("live observability stack is not health-checked and smoke-proven"),
        );
    }
    if let Err(failure) = command_inventory_complete(root) {
        return base_receipt(root, command, "fail", Some(&failure));
    }
    base_receipt(root, command, "pass", None)
}

fn command_inventory_complete(root: &Path) -> Result<(), String> {
    let failures = crate::audit::observability::command_inventory_failures(root);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(command_inventory_failure_summary(root, &failures))
    }
}

fn command_inventory_failure_summary(root: &Path, failures: &[String]) -> String {
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .unwrap_or(Value::Null);
    let board = inventory
        .get("observability_control_board")
        .unwrap_or(&Value::Null);
    let first_incomplete = board.get("first_incomplete").unwrap_or(&Value::Null);
    format!(
        "observability command inventory incomplete: status={} total_failures={} first_failure={} control_board_first_family={} control_board_first_incomplete={} control_board_first_status={} next_unobservable_surface={} family_counts={}",
        text_field(board, "status", "unknown"),
        failures.len(),
        failures.first().map(String::as_str).unwrap_or("none"),
        text_field(first_incomplete, "family", "unknown"),
        text_field(first_incomplete, "id", "unknown"),
        text_field(first_incomplete, "observability_status", "unknown"),
        text_field(first_incomplete, "next_unobservable_surface", "unknown"),
        family_counts(board)
    )
}

fn family_counts(board: &Value) -> String {
    let Some(families) = board.get("families").and_then(Value::as_object) else {
        return "unavailable".to_string();
    };
    let mut rows = families
        .iter()
        .map(|(family, row)| {
            format!(
                "{}={}/{}/{}/{}",
                family,
                count_field(row, "total"),
                count_field(row, "observable"),
                count_field(row, "partially_observable"),
                count_field(row, "unobservable")
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows.join(",")
}

fn count_field(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or(0)
}

fn text_field<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}

pub(crate) fn query_result(
    root: &Path,
    command: &ObserveCommand,
    query_kind: &str,
    query_text: String,
    rows: Vec<Value>,
    status: &str,
    failure: Option<&str>,
) -> Result<Value, String> {
    query::result(root, command, query_kind, query_text, rows, status, failure)
}

pub(crate) fn metric_summary(rows: &[Value]) -> Value {
    query::metric_summary("metrics", rows)
}

pub(crate) fn command_receipt(
    root: &Path,
    input: command::CommandTelemetry<'_>,
) -> Result<Value, String> {
    command::receipt(root, input)
}

pub(crate) fn command_receipt_for_candidate(
    root: &Path,
    input: command::CommandTelemetry<'_>,
    candidate: String,
) -> Result<Value, String> {
    command::receipt_for_candidate(root, input, candidate)
}

pub(crate) use command::CommandTelemetry;
pub(crate) use runtime::RuntimeTelemetry;

pub(crate) fn live_stack_receipts_current(root: &Path, candidate: &str) -> bool {
    live_receipt_current(
        root,
        crate::cli::observe::types::ObserveOperation::StackHealth,
        candidate,
    ) && live_receipt_current(
        root,
        crate::cli::observe::types::ObserveOperation::StackSmoke,
        candidate,
    )
}

pub(crate) fn ensure_spool_dir(root: &Path) -> Result<(), String> {
    spool::ensure_dir(root).map(|_| ())
}

#[cfg(test)]
pub(crate) fn spool_write_for_test(root: &Path, event: &Value) -> Result<(), String> {
    spool::write(root, event)
}

#[cfg(test)]
pub(crate) fn spool_write_line_read_only_failure_for_test(path: &Path) -> String {
    spool::write_line_read_only_failure_for_test(path)
}

#[cfg(test)]
pub(crate) fn exporter_failure_probe_for_test() -> String {
    exporter::failed_post_message_for_test()
}

#[cfg(test)]
pub(crate) fn exporter_launch_failure_for_test() -> String {
    exporter::failed_launch_message_for_test()
}

#[cfg(test)]
pub(crate) fn exporter_metric_line_for_test(metric: &Value) -> String {
    exporter::metric_line_for_test(metric)
}

#[cfg(test)]
pub(crate) fn exporter_trace_payload_for_test(trace: &Value) -> Value {
    exporter::trace_payload_for_test(trace)
}

#[cfg(test)]
pub(crate) fn query_receipt_text_for_test<'a>(
    telemetry: &'a Value,
    field: &str,
) -> Result<&'a str, String> {
    query::receipt_text_for_test(telemetry, field)
}

pub(crate) fn redact_sensitive_text(input: &str) -> String {
    record::redact_sensitive_text(input)
}

#[cfg(test)]
pub(crate) fn redacted_failure_for_test(input: &str) -> String {
    record::redacted_failure_for_test(input)
}

fn live_receipt_current(
    root: &Path,
    operation: crate::cli::observe::types::ObserveOperation,
    candidate: &str,
) -> bool {
    let path = root.join(operation.receipt_rel());
    let Ok(value) = crate::json_boundary::read_json(&path) else {
        return false;
    };
    if value.get("schema").and_then(Value::as_str)
        != Some(crate::cli::observe::types::RECEIPT_SCHEMA)
        || value.get("status").and_then(Value::as_str) != Some("pass")
        || value.get("candidate_digest").and_then(Value::as_str) != Some(candidate)
        || value.get("operation").and_then(Value::as_str) != Some(operation.id())
        || value.get("redaction_proof").and_then(Value::as_str) != Some("pass")
        || value.get("retention_bounds_proof").and_then(Value::as_str) != Some("pass")
        || value.get("bounded_output_proof").and_then(Value::as_str) != Some("pass")
    {
        return false;
    }
    let Some(event) = value.get("event") else {
        return false;
    };
    let Some(metric) = value.get("metric") else {
        return false;
    };
    let Some(trace) = value.get("trace") else {
        return false;
    };
    value.get("log_stream_digest").and_then(Value::as_str)
        == Some(crate::digest::canonical_json(event).as_str())
        && value.get("metric_snapshot_digest").and_then(Value::as_str)
            == Some(crate::digest::canonical_json(metric).as_str())
        && value.get("trace_bundle_digest").and_then(Value::as_str)
            == Some(crate::digest::canonical_json(trace).as_str())
}
