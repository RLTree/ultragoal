use serde_json::Value;
use std::path::Path;

mod cargo;
mod surface;

const CLI_SCHEMA: &str = "schemas/cli-control-plane-receipt.schema.json";
const SURFACE_SCHEMA: &str = "schemas/package-surface-audit-receipt.schema.json";
const CONTROL_SOURCE: &str = "validator/src/cli/control/plane/mod.rs";
const CONTROL_RECEIPT: &str = "validator/src/cli/control/plane/receipt.rs";
const CONTROL_SURFACE: &str = "validator/src/cli/control/plane/surface/mod.rs";
const CONTROL_TYPES: &str = "validator/src/cli/control/plane/types.rs";
const INSTALL_AUDIT_RECEIPT: &str = "validation_artifacts/cli/install-audit-receipt.json";
const CACHE_AUDIT_RECEIPT: &str = "validation_artifacts/cli/cache-audit-receipt.json";
const UPDATE_GOAL_RECEIPT: &str = "validation_artifacts/cli/update-goal-eligibility.json";
const SELF_LAW_RECEIPT: &str = "validation_artifacts/cli/self-law-receipt.json";
const REQUIRED_LAWS: &[&str] = &["cli-control-plane-authority", "cli-self-law-compliance"];
const REQUIRED_REDS: &[&str] = &[
    "cli-control-plane-authority-checklist-completion-without-cli-red",
    "cli-control-plane-authority-reviewer-agreement-bypass-red",
    "cli-control-plane-authority-source-audit-substituted-for-update-goal-red",
    "cli-self-law-compliance-bootstrap-receipt-used-for-completion-red",
    "cli-self-law-compliance-cli-source-excluded-from-coverage-red",
    "cli-self-law-compliance-update-goal-without-self-law-red",
];

pub fn package_failures(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    require_files(root, &mut out);
    require_cargo_bins(root, &mut out);
    require_manifest_resources(root, &mut out);
    require_schema_catalog(root, &mut out);
    require_law_rows(root, &mut out);
    require_red_catalog(root, &mut out);
    require_receipts(root, &mut out);
    out
}

fn require_files(root: &Path, out: &mut Vec<String>) {
    for rel in [
        CONTROL_SOURCE,
        CONTROL_RECEIPT,
        CONTROL_SURFACE,
        CONTROL_TYPES,
        CLI_SCHEMA,
        SURFACE_SCHEMA,
    ] {
        if !root.join(rel).is_file() {
            out.push(format!("cli_control_plane_missing_artifact:{rel}"));
        }
    }
}

fn require_cargo_bins(root: &Path, out: &mut Vec<String>) {
    let text = read_text(root, "validator/Cargo.toml");
    let bins = cargo::bin_names(&text);
    if !bins.contains("ultragoal") {
        out.push("cli_control_plane_missing_ultragoal_binary".to_string());
    }
    if bins.contains("ultragoal-validator") {
        out.push("cli_control_plane_legacy_validator_binary_present".to_string());
    }
}

fn require_manifest_resources(root: &Path, out: &mut Vec<String>) {
    let manifest = read_json(root, "plugin-manifest-draft.json");
    let inventory = crate::package::inventory::inventory_paths(&manifest);
    for rel in [
        CONTROL_SOURCE,
        CONTROL_RECEIPT,
        CONTROL_SURFACE,
        CONTROL_TYPES,
        CLI_SCHEMA,
        SURFACE_SCHEMA,
        INSTALL_AUDIT_RECEIPT,
        CACHE_AUDIT_RECEIPT,
        UPDATE_GOAL_RECEIPT,
        SELF_LAW_RECEIPT,
    ] {
        if !inventory.iter().any(|path| path == rel) {
            out.push(format!("cli_control_plane_package_inventory_missing:{rel}"));
        }
    }
}

fn require_schema_catalog(root: &Path, out: &mut Vec<String>) {
    let catalog = read_json(root, "schemas/schema-catalog.json");
    let Some(items) = catalog.get("schemas").and_then(Value::as_array) else {
        out.push("cli_control_plane_schema_catalog_unreadable".to_string());
        return;
    };
    if !items
        .iter()
        .any(|row| row.get("path").and_then(Value::as_str) == Some(CLI_SCHEMA))
    {
        out.push("cli_control_plane_schema_catalog_missing_receipt_schema".to_string());
    }
    if !items
        .iter()
        .any(|row| row.get("path").and_then(Value::as_str) == Some(SURFACE_SCHEMA))
    {
        out.push("cli_control_plane_schema_catalog_missing_surface_schema".to_string());
    }
}

fn require_law_rows(root: &Path, out: &mut Vec<String>) {
    for law in REQUIRED_LAWS {
        if !json_array_has_id(
            root,
            "templates/agent-standards/enforcement.json",
            "rows",
            law,
        ) {
            out.push(format!("cli_control_plane_missing_standards_row:{law}"));
        }
        if !json_array_has_id(
            root,
            "docs/source-obligation-matrix.json",
            "obligations",
            law,
        ) {
            out.push(format!("cli_control_plane_missing_source_obligation:{law}"));
        }
        if !trace_has_obligation(root, law) {
            out.push(format!(
                "cli_control_plane_missing_foundational_trace:{law}"
            ));
        }
        let valid = format!("fixtures/mandatory-law-surfaces/valid/{law}.json");
        if !root.join(&valid).is_file() {
            out.push(format!("cli_control_plane_missing_valid_fixture:{law}"));
        }
    }
}

fn require_red_catalog(root: &Path, out: &mut Vec<String>) {
    let catalog = read_json(root, "templates/RED_FIXTURES.json");
    for red in REQUIRED_REDS {
        if !array_contains_id(&catalog, red) {
            out.push(format!("cli_control_plane_missing_red_fixture:{red}"));
        }
    }
}

fn require_receipts(root: &Path, out: &mut Vec<String>) {
    let expected_candidate = match crate::package::inventory::package_digest(root) {
        Ok(digest) => digest,
        Err(err) => {
            out.push(format!(
                "cli_control_plane_candidate_digest_unavailable:{err}"
            ));
            return;
        }
    };
    for (rel, operation) in [
        (INSTALL_AUDIT_RECEIPT, "install_audit"),
        (CACHE_AUDIT_RECEIPT, "cache_audit"),
        (UPDATE_GOAL_RECEIPT, "update_goal_eligibility"),
        (SELF_LAW_RECEIPT, "self_update_goal_eligibility"),
    ] {
        match crate::json_boundary::read_json(&root.join(rel)) {
            Ok(value) => {
                let failures = if operation == "install_audit" {
                    surface::receipt_failures(
                        root,
                        &value,
                        &expected_candidate,
                        crate::cli::control::plane::types::ControlOperation::InstallAudit,
                    )
                } else if operation == "cache_audit" {
                    surface::receipt_failures(
                        root,
                        &value,
                        &expected_candidate,
                        crate::cli::control::plane::types::ControlOperation::CacheAudit,
                    )
                } else {
                    crate::cli::control::plane::receipt::same_candidate_fail_closed_failures(
                        &value,
                        &expected_candidate,
                        operation,
                    )
                };
                for failure in failures {
                    out.push(format!("{rel}: {failure}"));
                }
            }
            Err(_) => out.push(format!(
                "cli_control_plane_missing_fail_closed_receipt:{rel}"
            )),
        }
    }
}

fn read_text(root: &Path, rel: &str) -> String {
    crate::digest::read_file_bytes(&root.join(rel))
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .unwrap_or_default()
}

fn read_json(root: &Path, rel: &str) -> Value {
    crate::json_boundary::read_json(&root.join(rel)).unwrap_or(Value::Null)
}

fn json_array_has_id(root: &Path, rel: &str, key: &str, id: &str) -> bool {
    read_json(root, rel)
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|rows| rows.iter().any(|row| has_id(row, id)))
}

fn trace_has_obligation(root: &Path, id: &str) -> bool {
    read_json(root, "docs/foundational-law-traceability.json")
        .get("entries")
        .and_then(Value::as_array)
        .is_some_and(|rows| rows.iter().any(|row| has_obligation_id(row, id)))
}

fn array_contains_id(value: &Value, id: &str) -> bool {
    value
        .as_array()
        .is_some_and(|rows| rows.iter().any(|row| has_id(row, id)))
}

fn has_id(row: &Value, id: &str) -> bool {
    row.get("id").and_then(Value::as_str) == Some(id)
}

fn has_obligation_id(row: &Value, id: &str) -> bool {
    row.get("obligation_id").and_then(Value::as_str) == Some(id)
}
