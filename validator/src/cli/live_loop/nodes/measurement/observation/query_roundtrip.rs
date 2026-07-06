use crate::cli::live_loop::surfaces::LoopValidationSurface;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const QUERY_TIMEOUT_MS: u64 = 3_000;
const METRIC_QUERY_TIMEOUT_MS: u64 = 60_000;
const TRACE_QUERY_TIMEOUT_MS: u64 = 30_000;
const ROW_LIMIT: usize = 100;
const BYTE_LIMIT: usize = 262_144;
const RECEIPT_DIR: &str = "validation_artifacts/observability/live-loop/commands";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RoundtripQuery {
    Logs,
    Metrics,
    Traces,
    ExplainFailure,
}

impl RoundtripQuery {
    fn operation(self) -> ObserveOperation {
        match self {
            Self::Logs => ObserveOperation::LogsQuery,
            Self::Metrics => ObserveOperation::MetricsQuery,
            Self::Traces => ObserveOperation::TracesQuery,
            Self::ExplainFailure => ObserveOperation::ExplainFailure,
        }
    }

    fn receipt_suffix(self) -> &'static str {
        match self {
            Self::Logs => "logs-query",
            Self::Metrics => "metrics-query",
            Self::Traces => "traces-query",
            Self::ExplainFailure => "explain-failure",
        }
    }

    fn timeout_ms(self) -> u64 {
        match self {
            Self::Metrics => METRIC_QUERY_TIMEOUT_MS,
            Self::Traces => TRACE_QUERY_TIMEOUT_MS,
            Self::Logs | Self::ExplainFailure => QUERY_TIMEOUT_MS,
        }
    }
}

pub(super) struct ObserveReceipt {
    pub(super) receipt: String,
    pub(super) exit_code: i32,
    pub(super) status: String,
    pub(super) value: Value,
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
    let value = crate::json_boundary::read_json(&root.join(&receipt))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("fail")
        .to_string();
    Ok(ObserveReceipt {
        receipt: receipt.display().to_string(),
        exit_code: code,
        status,
        value,
    })
}

pub(super) fn receipt_path(node_id: &str, suffix: &str) -> PathBuf {
    Path::new(RECEIPT_DIR).join(format!("{node_id}-{suffix}.json"))
}
