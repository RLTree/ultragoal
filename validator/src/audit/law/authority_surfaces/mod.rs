use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

mod registry;
mod source;

const BUILDER_CONTRACT_PREFIXES: &[&str] = &[
    "docs/ultragoal-contract-2026-07/",
    "docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md",
    "docs/parent-session-full-ultragoal-compliance-checklist-2026-06-25.md",
    "docs/parent-session-full-ultragoal-execution-spine-2026-06-30.md",
];

const REQUIRED_SURFACES: &[&str] = &[
    "docs/mandatory-law-surfaces.json",
    "templates/agent-standards/enforcement.json",
    "docs/source-obligation-matrix.json",
    "docs/foundational-law-traceability.json",
    "templates/RED_FIXTURES.json",
    "schemas/mandatory-law-surfaces.schema.json",
    "schemas/red-packet.schema.json",
    "plugin-manifest-draft.json",
];

pub(crate) fn package_failures(root: &Path) -> Vec<(String, String)> {
    let manifest = crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
        .unwrap_or(Value::Null);
    let inventory = crate::package::inventory::inventory_paths(&manifest)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let laws = crate::json_boundary::read_json(&root.join("docs/mandatory-law-surfaces.json"))
        .unwrap_or(Value::Null);
    let red_catalog = crate::json_boundary::read_json(&root.join("templates/RED_FIXTURES.json"))
        .unwrap_or(Value::Null);
    let mut out = Vec::new();
    out.extend(surface_inventory_failures(root, &inventory));
    out.extend(registry::law_registry_failures(
        root,
        &laws,
        &registry::red_catalog_ids(&red_catalog),
        &inventory,
    ));
    out.extend(source::source_text_failures(root));
    out.extend(generated_boundary_failures(root, &inventory));
    out
}

fn surface_inventory_failures(root: &Path, inventory: &BTreeSet<String>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for rel in REQUIRED_SURFACES {
        if !root.join(rel).is_file() {
            push(
                &mut out,
                "authority-source-binding",
                format!("authority_surface_missing:{rel}"),
            );
        }
        if !inventory.contains(*rel) {
            push(
                &mut out,
                "authority-source-binding",
                format!("authority_surface_not_in_package_inventory:{rel}"),
            );
        }
    }
    out
}

fn generated_boundary_failures(root: &Path, inventory: &BTreeSet<String>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for rel in inventory {
        if BUILDER_CONTRACT_PREFIXES
            .iter()
            .any(|prefix| rel.starts_with(prefix))
        {
            push(
                &mut out,
                "generated-proof-artifact-provenance-anti-fabrication",
                format!("builder_contract_file_in_package_evidence:{rel}"),
            );
        }
    }
    for rel in generated_files(root) {
        let value = crate::json_boundary::read_json(&root.join(&rel)).unwrap_or(Value::Null);
        if value
            .get("generated_from")
            .and_then(Value::as_str)
            .is_none()
            && value.pointer("/provenance/generated_from").is_none()
            && value.get("source_spec").is_none()
        {
            push(
                &mut out,
                "generated-proof-artifact-provenance-anti-fabrication",
                format!("generated_artifact_missing_provenance:{rel}"),
            );
        }
        if !inventory.contains(&rel) {
            push(
                &mut out,
                "generated-proof-artifact-provenance-anti-fabrication",
                format!("generated_artifact_not_in_package_inventory:{rel}"),
            );
        }
    }
    out
}

fn generated_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    collect_json_files(root, &root.join("docs/generated"), &mut out);
    out
}

fn collect_json_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(root, &path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json")
            && let Ok(rel) = path.strip_prefix(root)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn push(out: &mut Vec<(String, String)>, check: &str, detail: String) {
    out.push((check.to_string(), detail));
}

#[cfg(test)]
pub(crate) fn raw_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    source::raw_authority_failures_for_test(rel, text)
}

#[cfg(test)]
pub(crate) fn output_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    source::output_authority_failures_for_test(rel, text)
}
