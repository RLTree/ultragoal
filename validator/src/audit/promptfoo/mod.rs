use serde_json::Value;
use std::path::Path;

const RECEIPT_REL: &str = "validation_artifacts/promptfoo/adapter-receipt.json";
const SCHEMA: &str = "harness-ultragoal.promptfoo-adapter-receipt.v1";

#[cfg(test)]
mod tests;

pub(crate) fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let receipt = match crate::json_boundary::read_json(&root.join(RECEIPT_REL)) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!(
                "promptfoo_adapter_receipt_missing_or_malformed:{err}"
            ));
            return out;
        }
    };
    if receipt.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        out.push("promptfoo_adapter_receipt_wrong_schema".to_string());
    }
    out.extend(crate::cli::promptfoo::receipt_failures(root, &receipt));
    out
}
