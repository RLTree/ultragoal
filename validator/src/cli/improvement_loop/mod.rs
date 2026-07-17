use serde_json::Value;

mod evidence;
mod proof;
mod registry;

pub(crate) const LAW_ID: &str = "harness-improvement-loop-trace-feedback-eval-codex-handoff";
const RECEIPT_SCHEMA: &str = "harness-ultragoal.improvement-loop-receipt.v1";
const REGISTRY: &str = "docs/improvement-loop-registry.json";

pub(crate) fn receipt_failures(root: &std::path::Path, receipt: &Value) -> Vec<String> {
    proof::receipt_failures(root, receipt)
}
