mod diagnostics;
mod event;
#[cfg(test)]
mod event_receipt;
mod live_backend;
mod loop_run_snapshot;
mod observe_receipt_reader;
#[path = "../query/mod.rs"]
mod query;
#[cfg(test)]
mod receipts;
mod reconciliation_report;
#[cfg(test)]
mod status;
#[cfg(test)]
mod test_reconciliation;
#[cfg(test)]
mod validation_state_tests;

use super::full_command::FullCommandRun;
use crate::cli::live_loop::{LiveLoopCommand, surfaces::LoopValidationSurface};
pub(crate) use loop_run_snapshot::loop_run_snapshot_pending;
use serde_json::{Value, json};
use std::path::Path;
use std::thread::ScopedJoinHandle;
use std::time::Instant;
#[cfg(test)]
use test_reconciliation::reconcile_with_observe_roundtrip;

#[derive(Clone, Debug)]
pub(crate) struct TelemetryReconciliation {
    pub(crate) status: String,
    pub(crate) duration_ms: u64,
    pub(crate) value: Value,
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
    let started = Instant::now();
    let result = reconcile_result(root, surface, candidate, command, actual_work);
    let duration_ms = elapsed_ms(started);
    match result {
        Ok(report) => TelemetryReconciliation {
            status: report.status,
            duration_ms,
            value: with_duration(report.value, duration_ms),
        },
        Err(err) => TelemetryReconciliation {
            status: "command_observation_failed".to_string(),
            duration_ms,
            value: json!({
                "status": "command_observation_failed",
                "failure": err,
                "telemetry_reconciliation_duration_ms": duration_ms,
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
) -> Result<reconciliation_report::ReconciliationReport, String> {
    let observation_path = event::receipt_path(surface.id);
    let observation = event::write(
        root,
        surface,
        candidate,
        command,
        actual_work,
        &observation_path,
    )?;
    let roundtrips = run_bounded_query_roundtrips(
        root,
        surface,
        candidate,
        &observation.run_id,
        &observation.correlation_id,
    )?;
    Ok(reconciliation_report::from_roundtrips(
        observation,
        observation_path,
        roundtrips.logs,
        roundtrips.metrics,
        roundtrips.traces,
        roundtrips.explain,
    ))
}

struct ObserveRoundtrips {
    logs: query::ObserveReceipt,
    metrics: query::ObserveReceipt,
    traces: query::ObserveReceipt,
    explain: query::ObserveReceipt,
}

fn run_bounded_query_roundtrips(
    root: &Path,
    surface: LoopValidationSurface,
    candidate: &str,
    run_id: &str,
    correlation_id: &str,
) -> Result<ObserveRoundtrips, String> {
    std::thread::scope(|scope| {
        let logs = scope.spawn(|| {
            query::capture_query_roundtrip(
                root,
                surface,
                query::LiveQueryRoundtrip::Logs,
                run_id,
                correlation_id,
                candidate,
            )
        });
        let metrics = scope.spawn(|| {
            query::capture_query_roundtrip(
                root,
                surface,
                query::LiveQueryRoundtrip::Metrics,
                run_id,
                correlation_id,
                candidate,
            )
        });
        let traces = scope.spawn(|| {
            query::capture_query_roundtrip(
                root,
                surface,
                query::LiveQueryRoundtrip::Traces,
                run_id,
                correlation_id,
                candidate,
            )
        });
        let logs = join_query_capture("logs-query", logs)?;
        let metrics = join_query_capture("metrics-query", metrics)?;
        let traces = join_query_capture("traces-query", traces)?;
        let logs = query::write_pending_query_receipt(root, logs)?;
        let metrics = query::write_pending_query_receipt(root, metrics)?;
        let traces = query::write_pending_query_receipt(root, traces)?;
        let explain = query::run(
            root,
            surface,
            query::RoundtripQuery::ExplainFailure,
            run_id,
            correlation_id,
            candidate,
        )?;
        Ok(ObserveRoundtrips {
            logs,
            metrics,
            traces,
            explain,
        })
    })
}

fn join_query_capture<'scope>(
    label: &str,
    handle: ScopedJoinHandle<'scope, query::PendingQueryReceipt>,
) -> Result<query::PendingQueryReceipt, String> {
    Ok(handle
        .join()
        .map_err(|_| format!("observe {label} worker panicked"))?)
}

fn with_duration(mut value: Value, duration_ms: u64) -> Value {
    value
        .as_object_mut()
        .expect("reconciliation report projection is always a JSON object")
        .insert(
            "telemetry_reconciliation_duration_ms".to_string(),
            json!(duration_ms),
        );
    value
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}
