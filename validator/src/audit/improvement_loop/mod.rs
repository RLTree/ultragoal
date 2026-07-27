use serde_json::Value;
use std::path::Path;

const RECEIPT_REL: &str = "validation_artifacts/improvement-loop/loop-closure-receipt.json";
const SCHEMA: &str = "harness-ultragoal.improvement-loop-receipt.v1";
const REGISTRY: &str = "docs/improvement-loop-registry.json";
const REGISTRY_SCHEMA: &str = "harness-ultragoal.improvement-loop-registry.v1";
const LEARNING_SCHEMA: &str = "harness-ultragoal.learning-adoption.v1";
const PROJECTION_REL: &str = "docs/improvement-loop/knowledge-projection.md";
const PROJECTION_SCHEMA: &str = "harness-ultragoal.improvement-loop-knowledge-projection.v1";

mod knowledge;
mod knowledge_projection;
mod receipt;

#[cfg(test)]
mod tests;

pub(crate) fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let receipt_path = crate::output_path::literal_claim_artifact_path(
        root,
        RECEIPT_REL,
        "improvement loop closure receipt",
    );
    match crate::json_boundary::read_json(&receipt_path) {
        Ok(receipt) => {
            if receipt.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
                out.push("improvement_loop_receipt_wrong_schema".to_string());
            }
            out.extend(receipt::failures(root, &receipt));
        }
        Err(err) => out.push(format!(
            "improvement_loop_receipt_missing_or_malformed:{err}"
        )),
    }
    out.extend(knowledge::failures(root));
    out
}
