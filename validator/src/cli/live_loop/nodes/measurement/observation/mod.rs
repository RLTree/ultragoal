mod diagnostics;
mod event;
#[cfg(test)]
mod event_receipt;
mod query_roundtrip;
#[cfg(test)]
mod receipts;
#[cfg(test)]
mod status;

use super::full_command::FullCommandRun;
use crate::cli::live_loop::{LiveLoopCommand, surfaces::LoopValidationSurface};
use serde_json::{Value, json};
use std::path::Path;

#[derive(Clone, Debug)]
pub(crate) struct TelemetryReconciliation {
    pub(crate) status: String,
    pub(crate) value: Value,
}

#[derive(Clone, Debug)]
struct ReconciliationReport {
    status: String,
    value: Value,
}

impl TelemetryReconciliation {
    pub(crate) fn value(&self) -> Value {
        self.value.clone()
    }

    pub(crate) fn failure_summary(
        &self,
    ) -> crate::cli::live_loop::nodes::command_failure::CommandFailureSummary {
        crate::cli::live_loop::nodes::command_failure::CommandFailureSummary::from_value(
            self.value.get("command_observation"),
        )
    }
}

pub(crate) fn reconcile(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    command: &LiveLoopCommand,
    actual_work: &FullCommandRun,
) -> TelemetryReconciliation {
    match reconcile_result(root, surface, candidate, command, actual_work) {
        Ok(report) => TelemetryReconciliation {
            status: report.status,
            value: report.value,
        },
        Err(err) => TelemetryReconciliation {
            status: "command_observation_failed".to_string(),
            value: json!({
                "status": "command_observation_failed",
                "failure": err,
                "claim_impact": "live_loop_node_timing_blocked"
            }),
        },
    }
}

fn reconcile_result(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    command: &LiveLoopCommand,
    actual_work: &FullCommandRun,
) -> Result<ReconciliationReport, String> {
    reconcile_with_observe_roundtrip(
        root,
        surface,
        candidate,
        command,
        actual_work,
        &mut |roundtrip, run_id, correlation_id| {
            query_roundtrip::run(root, surface, roundtrip, run_id, correlation_id)
        },
    )
}

fn reconcile_with_observe_roundtrip<F>(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    command: &LiveLoopCommand,
    actual_work: &FullCommandRun,
    roundtrip: &mut F,
) -> Result<ReconciliationReport, String>
where
    F: FnMut(
        query_roundtrip::RoundtripQuery,
        &str,
        &str,
    ) -> Result<query_roundtrip::ObserveReceipt, String>,
{
    let observation_path = event::receipt_path(surface.id);
    let observation = event::write(
        root,
        surface,
        candidate,
        command,
        actual_work,
        &observation_path,
    )?;
    let logs = roundtrip(
        query_roundtrip::RoundtripQuery::Logs,
        &observation.run_id,
        &observation.correlation_id,
    )?;
    let traces = roundtrip(
        query_roundtrip::RoundtripQuery::Traces,
        &observation.run_id,
        &observation.correlation_id,
    )?;
    let metrics = roundtrip(
        query_roundtrip::RoundtripQuery::Metrics,
        &observation.run_id,
        &observation.correlation_id,
    )?;
    let explain = roundtrip(
        query_roundtrip::RoundtripQuery::ExplainFailure,
        &observation.run_id,
        &observation.correlation_id,
    )?;
    let status = reconciliation_status(all_roundtrip_statuses_pass(
        &logs, &metrics, &traces, &explain,
    ));
    let run_id = observation.run_id.clone();
    let correlation_id = observation.correlation_id.clone();
    let trace_id = observation.trace_id.clone();
    let value = json!({
        "status": status,
        "run_id": run_id,
        "correlation_id": correlation_id,
        "trace_id": trace_id,
        "command_observation_receipt": observation_path.display().to_string(),
        "command_observation": observation.value(),
        "logs_query": logs.value(),
        "metrics_query": metrics.value(),
        "traces_query": traces.value(),
        "explain_failure": explain.value(),
        "claim_impact": "source_local_live_loop_node_observation_only_not_speed_claim"
    });
    Ok(ReconciliationReport {
        status: status.to_string(),
        value,
    })
}

fn all_roundtrip_statuses_pass(
    logs: &query_roundtrip::ObserveReceipt,
    metrics: &query_roundtrip::ObserveReceipt,
    traces: &query_roundtrip::ObserveReceipt,
    explain: &query_roundtrip::ObserveReceipt,
) -> bool {
    query_receipt_is_claim_observable(logs)
        && query_receipt_is_claim_observable(metrics)
        && query_receipt_is_claim_observable(traces)
        && explain_receipt_is_claim_observable(explain)
}

fn query_receipt_is_claim_observable(receipt: &query_roundtrip::ObserveReceipt) -> bool {
    receipt.exit_code == 0
        && receipt.status == "pass"
        && text(&receipt.value, "status") == Some("pass")
        && text(&receipt.value, "bounded_output_status") == Some("pass")
        && text(&receipt.value, "redaction_status") == Some("pass")
        && receipt
            .value
            .get("row_count")
            .and_then(Value::as_u64)
            .is_some_and(|rows| rows > 0)
        && text(&receipt.value, "result_digest").is_some_and(nonempty)
}

fn explain_receipt_is_claim_observable(receipt: &query_roundtrip::ObserveReceipt) -> bool {
    receipt.exit_code == 0
        && receipt.status == "pass"
        && text(&receipt.value, "status") == Some("pass")
        && text(&receipt.value, "bounded_output_proof") == Some("pass")
        && text(&receipt.value, "failure_class").is_some()
        && text(&receipt.value, "claim_impact").is_some_and(nonempty)
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn nonempty(value: &str) -> bool {
    !value.is_empty()
}

fn reconciliation_status(roundtrips_passed: bool) -> &'static str {
    match roundtrips_passed {
        true => "pass",
        false => "query_or_explain_reconciliation_failed",
    }
}
