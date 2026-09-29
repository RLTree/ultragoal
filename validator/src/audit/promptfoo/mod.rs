use serde_json::Value;
use std::path::Path;

const RECEIPT_REL: &str = "validation_artifacts/promptfoo/adapter-receipt.json";
const SCHEMA: &str = "harness-ultragoal.promptfoo-adapter-receipt.v1";
const PROMPTFOO_VERSION: &str = "0.121.17";

mod receipt;

#[cfg(test)]
mod tests;

pub(crate) fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let receipt_path = crate::output_path::literal_claim_artifact_path(
        root,
        RECEIPT_REL,
        "promptfoo adapter receipt",
    );
    let receipt = match crate::json_boundary::read_json(&receipt_path) {
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
    out.extend(receipt::failures(root, &receipt));
    out
}
