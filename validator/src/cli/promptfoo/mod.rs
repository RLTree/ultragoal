use serde_json::Value;

mod proof;
mod registry;

pub(crate) const LAW_ID: &str = "promptfoo-eval-red-team-provider-separation";
pub(super) const RECEIPT_SCHEMA: &str = "harness-ultragoal.promptfoo-adapter-receipt.v1";
pub(super) const PACKAGE_JSON: &str = "package.json";
pub(super) const PNPM_LOCK: &str = "pnpm-lock.yaml";
pub(super) const PNPM_WORKSPACE: &str = "pnpm-workspace.yaml";
pub(super) const PROVIDER_REGISTRY: &str = "docs/promptfoo-provider-registry.json";
pub(super) const SUITE_REGISTRY: &str = "docs/promptfoo-suite-registry.json";
pub(super) const PROMPTFOO_VERSION: &str = "0.121.17";

pub(crate) fn receipt_failures(root: &std::path::Path, receipt: &Value) -> Vec<String> {
    proof::receipt_failures(root, receipt)
}
