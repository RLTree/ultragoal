use crate::cli::live_loop::surfaces::LoopValidationSurface;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const QUERY_TIMEOUT_MS: u64 = 3_000;
const TRACE_QUERY_TIMEOUT_MS: u64 = 10_000;
const ROW_LIMIT: usize = 100;
const BYTE_LIMIT: usize = 262_144;
const RECEIPT_DIR: &str = "validation_artifacts/observability/live-loop/commands";

pub(super) struct ObserveReceipt {
    pub(super) receipt: String,
    pub(super) exit_code: i32,
    pub(super) status: String,
    value: Value,
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
    operation: ObserveOperation,
    run_id: &str,
    correlation_id: &str,
) -> Result<ObserveReceipt, String> {
    let receipt = receipt_path(surface.id, receipt_suffix(operation));
    let command = ObserveCommand {
        operation,
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
        timeout_ms: timeout_ms(operation),
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

fn receipt_suffix(operation: ObserveOperation) -> &'static str {
    match operation {
        ObserveOperation::LogsQuery => "logs-query",
        ObserveOperation::MetricsQuery => "metrics-query",
        ObserveOperation::TracesQuery => "traces-query",
        ObserveOperation::ExplainFailure => "explain-failure",
        _ => "observe",
    }
}

fn timeout_ms(operation: ObserveOperation) -> u64 {
    if operation == ObserveOperation::TracesQuery {
        TRACE_QUERY_TIMEOUT_MS
    } else {
        QUERY_TIMEOUT_MS
    }
}
