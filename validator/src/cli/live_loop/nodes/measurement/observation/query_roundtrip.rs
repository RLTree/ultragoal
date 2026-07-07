use crate::cli::live_loop::surfaces::LoopValidationSurface;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::{Value, json};
use std::path::Path;

const QUERY_TIMEOUT_MS: u64 = 3_000;
const METRIC_QUERY_TIMEOUT_MS: u64 = 30_000;
const TRACE_QUERY_TIMEOUT_MS: u64 = 30_000;
pub(super) const ROW_LIMIT: usize = 100;
pub(super) const BYTE_LIMIT: usize = 262_144;
pub(super) const RECEIPT_DIR: &str = "validation_artifacts/observability/live-loop/commands";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RoundtripQuery {
    Logs,
    Metrics,
    Traces,
    ExplainFailure,
}

impl RoundtripQuery {
    pub(super) fn operation(self) -> ObserveOperation {
        match self {
            Self::Logs => ObserveOperation::LogsQuery,
            Self::Metrics => ObserveOperation::MetricsQuery,
            Self::Traces => ObserveOperation::TracesQuery,
            Self::ExplainFailure => ObserveOperation::ExplainFailure,
        }
    }

    pub(super) fn receipt_suffix(self) -> &'static str {
        match self {
            Self::Logs => "logs-query",
            Self::Metrics => "metrics-query",
            Self::Traces => "traces-query",
            Self::ExplainFailure => "explain-failure",
        }
    }

    pub(super) fn timeout_ms(self) -> u64 {
        match self {
            Self::Metrics => METRIC_QUERY_TIMEOUT_MS,
            Self::Traces => TRACE_QUERY_TIMEOUT_MS,
            Self::Logs | Self::ExplainFailure => QUERY_TIMEOUT_MS,
        }
    }
}

#[derive(Debug)]
pub(super) struct ObserveReceipt {
    pub(super) receipt: String,
    pub(super) exit_code: i32,
    pub(super) status: String,
    pub(super) value: Value,
}

#[derive(Clone, Copy)]
enum BackendState {
    NotRequired,
    Ready,
    Unavailable(super::backend_readiness::LiveBackend),
}

impl ObserveReceipt {
    pub(super) fn value(&self) -> Value {
        json!({
            "receipt": self.receipt,
            "exit_code": self.exit_code,
            "status": self.status,
            "value": self.value
        })
    }
}

pub(super) fn run(
    root: &Path,
    surface: LoopValidationSurface,
    roundtrip: RoundtripQuery,
    run_id: &str,
    correlation_id: &str,
) -> Result<ObserveReceipt, String> {
    let receipt = receipt_path(surface.id, roundtrip.receipt_suffix());
    let state = backend_state(roundtrip);
    run_with_backend_state(
        root,
        surface,
        roundtrip,
        receipt,
        run_id,
        correlation_id,
        state,
    )
}

fn run_with_backend_state(
    root: &Path,
    surface: LoopValidationSurface,
    roundtrip: RoundtripQuery,
    receipt: std::path::PathBuf,
    run_id: &str,
    correlation_id: &str,
    state: BackendState,
) -> Result<ObserveReceipt, String> {
    if let Some(unavailable) = unavailable_backend_receipt(
        root,
        surface,
        roundtrip,
        &receipt,
        run_id,
        correlation_id,
        state,
    )? {
        return Ok(unavailable);
    }
    run_observe_query(root, roundtrip, receipt, run_id, correlation_id)
}

fn backend_state(roundtrip: RoundtripQuery) -> BackendState {
    match super::backend_readiness::backend_for(roundtrip) {
        Some(backend) => {
            backend_state_from_probe(backend, super::backend_readiness::ready(backend))
        }
        None => BackendState::NotRequired,
    }
}

fn backend_state_from_probe(
    backend: super::backend_readiness::LiveBackend,
    ready: bool,
) -> BackendState {
    match ready {
        true => BackendState::Ready,
        false => BackendState::Unavailable(backend),
    }
}

fn unavailable_backend_receipt(
    root: &Path,
    surface: LoopValidationSurface,
    roundtrip: RoundtripQuery,
    receipt: &std::path::Path,
    run_id: &str,
    correlation_id: &str,
    state: BackendState,
) -> Result<Option<ObserveReceipt>, String> {
    let BackendState::Unavailable(backend) = state else {
        return Ok(None);
    };
    let value = super::backend_readiness::write_unavailable_query_receipt(
        root,
        surface,
        roundtrip,
        backend,
        receipt,
        run_id,
        correlation_id,
    )?;
    Ok(Some(ObserveReceipt {
        receipt: receipt.display().to_string(),
        exit_code: 1,
        status: "fail".to_string(),
        value,
    }))
}

pub(super) fn receipt_path(node_id: &str, suffix: &str) -> std::path::PathBuf {
    super::backend_readiness::receipt_path(node_id, suffix)
}

fn run_observe_query(
    root: &Path,
    roundtrip: RoundtripQuery,
    receipt: std::path::PathBuf,
    run_id: &str,
    correlation_id: &str,
) -> Result<ObserveReceipt, String> {
    let command = ObserveCommand {
        operation: roundtrip.operation(),
        receipt: Some(receipt.clone()),
        query: None,
        run_id: Some(run_id.to_string()),
        correlation_id: Some(correlation_id.to_string()),
        claim_id: None,
        check_id: None,
        law_id: None,
        target_command: None,
        target_family: None,
        row_limit: ROW_LIMIT,
        byte_limit: BYTE_LIMIT,
        timeout_ms: roundtrip.timeout_ms(),
    };
    let code = crate::cli::observe::run(root, &command)?;
    read_observe_receipt(root, &receipt, roundtrip, code)
}

fn read_observe_receipt(
    root: &Path,
    receipt: &std::path::Path,
    roundtrip: RoundtripQuery,
    code: i32,
) -> Result<ObserveReceipt, String> {
    let path =
        crate::output_path::claim_artifact_path(root, receipt, "live loop observe query receipt")?;
    let value = crate::json_boundary::read_json(&path)?;
    let status = receipt_status(&value, roundtrip)?.to_string();
    Ok(ObserveReceipt {
        receipt: receipt.display().to_string(),
        exit_code: code,
        status,
        value,
    })
}

fn receipt_status<'a>(value: &'a Value, roundtrip: RoundtripQuery) -> Result<&'a str, String> {
    value.get("status").and_then(Value::as_str).ok_or_else(|| {
        format!(
            "observe {} receipt missing status",
            roundtrip.receipt_suffix()
        )
    })
}

#[cfg(test)]
#[path = "roundtrip_backend_tests.rs"]
mod tests;
