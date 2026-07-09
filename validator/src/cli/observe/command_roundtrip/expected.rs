use crate::audit::observability::specs::CommandObservabilitySpec;
use serde_json::Value;

pub(super) fn receipt_mismatches(receipt: &Value, spec: CommandObservabilitySpec) -> Vec<String> {
    let mut failures = Vec::new();
    if receipt.get("operation").and_then(Value::as_str) != Some(spec.operation) {
        failures.push("command_receipt_operation_mismatch".to_string());
    }
    if receipt.get("receipt_path").and_then(Value::as_str) != Some(spec.receipt_rel) {
        failures.push("command_receipt_path_mismatch".to_string());
    }
    failures
}
