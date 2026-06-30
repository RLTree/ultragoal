use crate::cli::observe::types::ObserveCommand;
use serde_json::Value;
use std::path::Path;

mod claims;
mod command;
mod exporter;
mod identity;
mod query;
mod receipt;
mod record;
mod spool;

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
    if let Err(failure) = fitting_inventory_complete(root) {
        return base_receipt(root, command, "fail", Some(&failure));
    }
    base_receipt(root, command, "pass", None)
}

fn fitting_inventory_complete(root: &Path) -> Result<(), String> {
    let failures = crate::audit::observability::command_fitting_failures(root);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "observability fitting inventory incomplete: {}",
            failures.join("; ")
        ))
    }
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

pub(crate) fn command_receipt(
    root: &Path,
    input: command::CommandTelemetry<'_>,
) -> Result<Value, String> {
    command::receipt(root, input)
}

pub(crate) use command::CommandTelemetry;

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
pub(crate) fn query_receipt_text_for_test<'a>(
    telemetry: &'a Value,
    field: &str,
) -> Result<&'a str, String> {
    query::receipt_text_for_test(telemetry, field)
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
