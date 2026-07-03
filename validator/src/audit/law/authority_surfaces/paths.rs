use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const CHECK_ID: &str = "authority-source-binding";

pub(super) fn require_path(
    root: &Path,
    inventory: &BTreeSet<String>,
    law: &str,
    role: &str,
    rel: &str,
    package_required: bool,
    out: &mut Vec<(String, String)>,
) {
    if crate::package::inventory::parent_session_contract_path(rel) {
        push(
            out,
            format!("builder_contract_path_used_as_law_surface:law={law};role={role};path={rel}"),
        );
        return;
    }
    if let Some(error) = crate::package::inventory::package_path_error(root, rel) {
        push(
            out,
            format!(
                "foundational_law_surface_path_invalid:law={law};role={role};path={rel};error={error}"
            ),
        );
        return;
    }
    if !root.join(rel).is_file() {
        push(
            out,
            format!("foundational_law_surface_path_missing:law={law};role={role};path={rel}"),
        );
    }
    if package_required && !inventory.contains(rel) {
        push(
            out,
            format!("foundational_law_surface_path_not_packaged:law={law};role={role};path={rel}"),
        );
    }
}

pub(super) fn runtime_artifact(rel: &str) -> bool {
    rel.starts_with("validation_artifacts/")
}

pub(super) fn read_json(root: &Path, rel: &str) -> Value {
    crate::json_boundary::read_json(&root.join(rel)).unwrap_or(Value::Null)
}

fn push(out: &mut Vec<(String, String)>, detail: String) {
    out.push((CHECK_ID.to_string(), detail));
}
