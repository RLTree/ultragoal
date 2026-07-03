use std::path::Path;

pub(super) fn source_text_failures(root: &Path) -> Vec<(String, String)> {
    actual_source_files(root)
        .into_iter()
        .flat_map(|rel| {
            let text = std::fs::read_to_string(root.join(&rel)).unwrap_or_default();
            let mut out = raw_authority_failures_for_text(&rel, &text)
                .into_iter()
                .map(|failure| ("typed-records-over-prose".to_string(), failure))
                .collect::<Vec<_>>();
            out.extend(
                output_authority_failures_for_text(&rel, &text)
                    .into_iter()
                    .map(|failure| {
                        (
                            "total-authority-types-impossible-state-elimination".to_string(),
                            failure,
                        )
                    }),
            );
            out
        })
        .collect()
}

fn raw_authority_failures_for_text(rel: &str, text: &str) -> Vec<String> {
    if !contains_raw_authority_marker(text) || raw_authority_class(rel).is_some() {
        return Vec::new();
    }
    vec![format!(
        "raw_downstream_authority_unclassified:path={rel};classification_required=parser_boundary|projection|fixture_catalog_materialization|catalog_materialization;repair=route_raw_input_through_typed_record_or_typed_failure_before_law_execution"
    )]
}

fn output_authority_failures_for_text(rel: &str, text: &str) -> Vec<String> {
    if rel.contains("/self_tests/")
        || rel.contains("/tests/")
        || rel.ends_with("/tests.rs")
        || rel.contains("/test_")
    {
        return Vec::new();
    }
    if allowed_direct_receipt_path(rel) || text.contains("claim_artifact_path") {
        return Vec::new();
    }
    [
        "receipt.to_path_buf()",
        "command.receipt.clone()",
        "resolve(root, &command.receipt)",
        "root.join(&command.receipt)",
        "root.join(receipt)",
    ]
    .into_iter()
    .filter(|pattern| text.contains(pattern))
    .map(|pattern| {
        format!(
            "claim_artifact_output_without_typed_authority:path={rel};pattern={pattern};repair=use_output_path_claim_artifact_path_or_mark_external_debug_no_claim"
        )
    })
    .collect()
}

fn contains_raw_authority_marker(text: &str) -> bool {
    text.contains("serde_json::Value")
        || text.contains("use serde_json::Value")
        || text.contains("serde_json::Map")
        || text.contains("BTreeMap<String, Value")
        || text.contains("HashMap<String, Value")
        || text.contains("\"raw_")
        || text.contains("raw_observation")
}

fn raw_authority_class(rel: &str) -> Option<&'static str> {
    if rel.contains("/self_tests/") || rel.contains("/tests/") || rel.ends_with("/tests.rs") {
        return Some("fixture_catalog_materialization");
    }
    if rel.contains("/stdout")
        || rel.contains("/emit")
        || rel.contains("/receipt")
        || rel.contains("/telemetry")
    {
        return Some("projection");
    }
    if rel.contains("/red/") || rel.contains("/fixtures/") || rel.contains("/package/schema/") {
        return Some("fixture_catalog_materialization");
    }
    if rel.starts_with("validator/src/cli/")
        || rel.starts_with("validator/src/audit/")
        || rel.starts_with("validator/src/package/")
        || rel.starts_with("validator/src/claim_semantics/")
        || rel.starts_with("validator/src/review/")
        || rel.starts_with("validator/src/target_repo/")
        || rel == "validator/src/json_boundary.rs"
        || rel == "validator/src/schema_catalog.rs"
    {
        return Some("parser_boundary");
    }
    None
}

fn allowed_direct_receipt_path(rel: &str) -> bool {
    matches!(
        rel,
        "validator/src/cli/standards/gardener.rs"
            | "validator/src/cli/session.rs"
            | "validator/src/cli/coverage/receipt_fields.rs"
            | "validator/src/cli/control/plane/transactional/telemetry/mod.rs"
    )
}

fn actual_source_files(root: &Path) -> Vec<String> {
    crate::package::inventory::closure::actual_files(root)
        .unwrap_or_default()
        .into_iter()
        .filter(|rel| rel.starts_with("validator/src/") && rel.ends_with(".rs"))
        .filter(|rel| !rel.contains("/self_tests/"))
        .collect()
}

#[cfg(test)]
pub(crate) fn raw_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    raw_authority_failures_for_text(rel, text)
}

#[cfg(test)]
pub(crate) fn output_authority_failures_for_test(rel: &str, text: &str) -> Vec<String> {
    output_authority_failures_for_text(rel, text)
}
