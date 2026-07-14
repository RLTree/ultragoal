use crate::cli::observe::command::ObserveCommand;
use serde_json::Value;
use std::path::Path;

mod claims;
mod command;
mod exporter;
mod identity;
mod inventory_status;
mod metric;
mod query;
#[cfg(test)]
mod query_receipt_tests;
mod receipt;
mod record;
mod runtime;
mod spool;
mod stack_receipt;
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

pub(crate) fn base_receipt_for_candidate(
    root: &Path,
    command: &ObserveCommand,
    status: &str,
    failure: Option<&str>,
    candidate: String,
) -> Result<Value, String> {
    receipt::base_for_candidate(root, command, status, failure, candidate)
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
    if let Err(failure) = inventory_status::complete(root) {
        return base_receipt(root, command, "fail", Some(&failure));
    }
    base_receipt(root, command, "pass", None)
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

pub(crate) fn query_result_for_candidate(
    root: &Path,
    command: &ObserveCommand,
    query_kind: &str,
    query_text: String,
    rows: Vec<Value>,
    status: &str,
    failure: Option<&str>,
    candidate: String,
) -> Result<Value, String> {
    query::result_for_candidate(
        root, command, query_kind, query_text, rows, status, failure, candidate,
    )
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
    stack_receipt::current(root, candidate)
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
