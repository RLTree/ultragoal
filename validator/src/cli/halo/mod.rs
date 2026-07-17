use serde_json::Value;

mod proof;

pub(crate) const LAW_ID: &str = "halo-ranked-harness-change-optimization";
const REGISTRY: &str = "docs/halo-adapter-registry.json";
const SCHEMA: &str = "harness-ultragoal.halo-capability-receipt.v1";

pub(crate) fn receipt_failures(root: &std::path::Path, receipt: &Value) -> Vec<String> {
    proof::receipt_failures(root, receipt)
}
