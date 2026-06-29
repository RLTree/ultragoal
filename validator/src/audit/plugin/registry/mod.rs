use crate::{json_boundary, schema_catalog};
use serde_json::Value;
use std::path::Path;

mod guard;
mod live;

const REGISTRY_RECEIPT: &str =
    "validation_artifacts/ultragoal-audit/active-registry-exposure-current.json";

pub(crate) fn failures(root: &Path, store: &schema_catalog::SchemaStore) -> Vec<String> {
    let mut out = Vec::new();
    let Some(receipt) = read(root, REGISTRY_RECEIPT, &mut out) else {
        return out;
    };
    out.extend(value_failures(root, store, &receipt));
    out
}

pub(crate) fn claim_guard_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
) -> Vec<String> {
    let mut out = Vec::new();
    let Some(receipt) = read(root, REGISTRY_RECEIPT, &mut out) else {
        return out;
    };
    out.extend(value_claim_guard_failures(root, store, &receipt));
    out
}

pub(crate) fn value_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    receipt: &Value,
) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(
        schema_catalog::schema_errors(store, "codex-registry-exposure.schema.json", &receipt)
            .into_iter()
            .map(|err| format!("plugin_self_law_registry_schema:{err}")),
    );
    out.extend(live::failures(root, receipt));
    out
}

pub(crate) fn value_claim_guard_failures(
    root: &Path,
    store: &schema_catalog::SchemaStore,
    receipt: &Value,
) -> Vec<String> {
    if receipt.get("status").and_then(Value::as_str) == Some("pass") {
        return value_failures(root, store, receipt);
    }
    let mut out = Vec::new();
    out.extend(
        schema_catalog::schema_errors(store, "codex-registry-exposure.schema.json", &receipt)
            .into_iter()
            .map(|err| format!("plugin_self_law_registry_schema:{err}")),
    );
    out.extend(guard::failures(root, receipt));
    out
}

fn read(root: &Path, rel: &str, out: &mut Vec<String>) -> Option<Value> {
    match json_boundary::read_json(&root.join(rel)) {
        Ok(value) => Some(value),
        Err(err) => {
            out.push(format!(
                "plugin_self_law_json_missing_or_malformed:{rel}:{err}"
            ));
            None
        }
    }
}
