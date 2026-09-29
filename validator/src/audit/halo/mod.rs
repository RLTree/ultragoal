use serde_json::Value;
use std::path::Path;

const RECEIPT: &str = "validation_artifacts/halo/capability-receipt.json";
const SCHEMA: &str = "harness-ultragoal.halo-capability-receipt.v1";
const REGISTRY: &str = "docs/halo-adapter-registry.json";

mod receipt;

#[cfg(test)]
mod tests;

pub(crate) fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let receipt = match crate::json_boundary::read_json(&root.join(RECEIPT)) {
        Ok(value) => value,
        Err(err) => {
            out.push(format!(
                "halo_capability_receipt_missing_or_malformed:{err}"
            ));
            return out;
        }
    };
    if receipt.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        out.push("halo_capability_receipt_wrong_schema".to_string());
    }
    out.extend(receipt::failures(root, &receipt));
    out
}
