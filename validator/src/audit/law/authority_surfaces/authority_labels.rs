use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const CHECK_ID: &str = "generated-proof-artifact-provenance-anti-fabrication";

pub(super) fn package_json_label_failures(
    root: &Path,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut surfaces = inventory
        .iter()
        .filter(|rel| package_json_authority_surface(rel))
        .cloned()
        .collect::<BTreeSet<_>>();
    surfaces.extend(
        super::inventory_requirements::required_surfaces()
            .iter()
            .filter(|surface| required_json_authority_surface(surface.rel))
            .map(|surface| surface.rel.to_string()),
    );
    for rel in surfaces {
        if !root.join(&rel).is_file() {
            continue;
        }
        let value = crate::json_boundary::read_json(&root.join(&rel)).unwrap_or(Value::Null);
        collect_label_failures(&rel, "$", &value, &mut out);
    }
    out
}

fn package_json_authority_surface(rel: &str) -> bool {
    rel.ends_with(".json")
        && !red_fixture_surface(rel)
        && !schema_or_catalog_surface(rel)
        && !crate::package::inventory::builder_contract_resource_path(rel)
        && (rel.starts_with("docs/")
            || rel.starts_with(".harness/")
            || rel.starts_with("fixtures/mandatory-law-surfaces/valid/")
            || rel == "plugin-manifest-draft.json"
            || rel.starts_with("templates/agent-standards/"))
}

fn required_json_authority_surface(rel: &str) -> bool {
    package_json_authority_surface(rel)
        || (rel.ends_with(".json")
            && !red_fixture_surface(rel)
            && !schema_or_catalog_surface(rel)
            && !crate::package::inventory::builder_contract_resource_path(rel)
            && rel.starts_with("validation_artifacts/"))
}

fn red_fixture_surface(rel: &str) -> bool {
    rel.starts_with("fixtures/red/")
        || rel.contains("/red/")
        || rel.starts_with("fixtures/target-repo/red/")
}

fn schema_or_catalog_surface(rel: &str) -> bool {
    rel.starts_with("schemas/")
        || rel == "templates/RED_FIXTURES.json"
        || rel.contains("red-fixture")
        || rel.contains("red_fixtures")
}

fn collect_label_failures(
    rel: &str,
    pointer: &str,
    value: &Value,
    out: &mut Vec<(String, String)>,
) {
    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_label_failures(rel, &format!("{pointer}/{index}"), item, out);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                collect_label_failures(rel, &format!("{pointer}/{key}"), item, out);
            }
        }
        Value::String(text) => {
            if !authority_pointer(pointer) {
                return;
            }
            if let Some(label) =
                crate::audit::namespace::source::path_labels::product_opaque_goal_work_string_label(
                    text,
                )
            {
                out.push((
                    CHECK_ID.to_string(),
                    format!(
                        "package_json_authority_product_opaque_label:{rel}:{pointer}:label={label}"
                    ),
                ));
            }
        }
        _ => {}
    }
}

fn authority_pointer(pointer: &str) -> bool {
    let Some(key) = pointer.rsplit('/').find(|part| {
        !part.is_empty() && *part != "$" && !part.chars().all(|ch| ch.is_ascii_digit())
    }) else {
        return false;
    };
    let lower = key.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "id" | "ids"
            | "path"
            | "paths"
            | "command"
            | "commands"
            | "artifact"
            | "artifacts"
            | "receipt"
            | "receipts"
            | "surface"
            | "surfaces"
            | "claim"
            | "claims"
            | "claim_impact"
            | "claim_ceiling"
            | "claim_ceiling_impact"
            | "blocked_claims"
            | "supported_claims"
            | "unsupported_claims"
            | "generated_from"
            | "source_spec"
            | "source_artifact_method"
    ) || lower.ends_with("_id")
        || lower.ends_with("_ids")
        || lower.ends_with("_path")
        || lower.ends_with("_paths")
        || lower.ends_with("_command")
        || lower.ends_with("_commands")
        || lower.ends_with("_artifact")
        || lower.ends_with("_artifacts")
        || lower.ends_with("_receipt")
        || lower.ends_with("_receipts")
        || lower.ends_with("_surface")
        || lower.ends_with("_surfaces")
        || lower.ends_with("_claim")
        || lower.ends_with("_claims")
        || lower.ends_with("_impact")
        || lower.ends_with("_ceiling")
}
