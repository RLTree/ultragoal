use serde_json::Value;
use std::path::Path;

pub use crate::audit::law::surface::receipt::runtime::{
    product_live_surface_value_failures, runtime_tool_identity_value_failures,
    transcript_quality_value_failures,
};
pub use crate::audit::law::surface::receipt::workflow::{
    clean_checkout_value_failures, memory_context_value_failures,
    restartable_execplan_value_failures,
};

const VALID_RECEIPTS: &[(&str, fn(&Path, &Value) -> Vec<String>)] = &[
    (
        "fixtures/law-surfaces/valid/runtime-tool-identity-receipt.json",
        runtime_tool_identity_value_failures,
    ),
    (
        "fixtures/law-surfaces/valid/product-live-surface-receipt.json",
        product_live_surface_value_failures,
    ),
    (
        "fixtures/law-surfaces/valid/transcript-quality-receipt.json",
        transcript_quality_value_failures,
    ),
    (
        "fixtures/law-surfaces/valid/clean-checkout-command-discovery-receipt.json",
        clean_checkout_value_failures,
    ),
    (
        "fixtures/law-surfaces/valid/restartable-execplan-receipt.json",
        restartable_execplan_value_failures,
    ),
    (
        "fixtures/law-surfaces/valid/memory-context-boundary-receipt.json",
        memory_context_value_failures,
    ),
];

pub fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for (path, check) in VALID_RECEIPTS {
        match crate::json_boundary::read_json(&root.join(path)) {
            Ok(value) => out.extend(
                check(root, &value)
                    .into_iter()
                    .map(|failure| format!("{path}: {failure}")),
            ),
            Err(err) => out.push(format!("{path}: {err}")),
        }
    }
    out
}
