use super::query::RoundtripQuery;
use serde_json::{Value, json};
use std::path::Path;

#[derive(Debug)]
pub(in crate::cli::live_loop::nodes::measurement::observation) struct ObserveReceipt {
    pub(in crate::cli::live_loop::nodes::measurement::observation) receipt: String,
    pub(in crate::cli::live_loop::nodes::measurement::observation) exit_code: i32,
    pub(in crate::cli::live_loop::nodes::measurement::observation) status: String,
    pub(in crate::cli::live_loop::nodes::measurement::observation) duration_ms: u64,
    pub(in crate::cli::live_loop::nodes::measurement::observation) value: Value,
}

impl ObserveReceipt {
    pub(in crate::cli::live_loop::nodes::measurement::observation) fn value(&self) -> Value {
        json!({
            "receipt": self.receipt,
            "exit_code": self.exit_code,
            "status": self.status,
            "duration_ms": self.duration_ms,
            "value": self.value
        })
    }
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn receipt_path(
    node_id: &str,
    suffix: &str,
) -> std::path::PathBuf {
    super::live_backend::receipt_path(node_id, suffix)
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn read_observe_receipt(
    root: &Path,
    receipt: &Path,
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
        duration_ms: 1,
        value,
    })
}

pub(in crate::cli::live_loop::nodes::measurement::observation) fn receipt_status<'a>(
    value: &'a Value,
    roundtrip: RoundtripQuery,
) -> Result<&'a str, String> {
    value.get("status").and_then(Value::as_str).ok_or_else(|| {
        format!(
            "observe {} receipt missing status",
            roundtrip.receipt_suffix()
        )
    })
}
