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
    let Some(marker) = raw_authority_marker(text) else {
        return Vec::new();
    };
    if raw_authority_class(rel, text).is_some() {
        return Vec::new();
    }
    vec![format!(
        "raw_downstream_authority_unclassified:path={rel};raw_authority={marker};classification_required=parser_boundary|projection|fixture_catalog_materialization|catalog_materialization;repair=route_raw_input_through_typed_record_or_typed_failure_before_law_execution"
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
        "PathBuf::from(&command.receipt)",
        "PathBuf::from(receipt)",
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

fn raw_authority_marker(text: &str) -> Option<&'static str> {
    if text.contains("serde_json::Map")
        || text.contains("BTreeMap<String, Value")
        || text.contains("HashMap<String, Value")
        || text.contains("raw_map")
    {
        return Some("raw_map");
    }
    if text.contains("raw_path") {
        return Some("raw_path");
    }
    if text.contains("raw_string") {
        return Some("raw_string");
    }
    if text.contains("serde_json::Value") || text.contains("use serde_json::Value") {
        return Some("raw_json");
    }
    if text.contains("\"raw_") || text.contains("raw_observation") {
        return Some("raw_observation");
    }
    None
}

fn raw_authority_class(rel: &str, text: &str) -> Option<&'static str> {
    if rel.contains("/self_tests/") || rel.contains("/tests/") || rel.ends_with("/tests.rs") {
        return Some("fixture_catalog_materialization");
    }
    if fixture_or_catalog_path(rel) {
        return Some("fixture_catalog_materialization");
    }
    if projection_boundary_text(text) {
        return Some("projection");
    }
    if parser_boundary_text(text) || typed_failure_boundary_text(text) {
        return Some("parser_boundary");
    }
    None
}

fn fixture_or_catalog_path(rel: &str) -> bool {
    rel.contains("/red/")
        || rel.contains("/fixtures/")
        || rel.contains("/fixture/")
        || rel.contains("/schema_catalog/")
        || rel.contains("/package/schema/")
        || rel.ends_with("schema_catalog.rs")
}

fn projection_boundary_text(text: &str) -> bool {
    text.contains("json!(")
        || text.contains("json!({")
        || text.contains("Value::Array(")
        || text.contains("Value::Object")
        || text.contains("serde_json::Map::new")
        || text.contains("serde_json::to_string")
        || text.contains("serde_json::to_value")
        || text.contains("canonical_json")
        || text.contains("stdout_contract")
        || text.contains("print_receipt")
        || text.contains("csv(")
        || text.contains("checks: &mut serde_json::Map")
        || text.contains("serde_json::Map<String, Value>")
        || text.contains("checks.insert(")
        || text.contains("metric.to_value(")
        || text.contains("FnOnce() -> Value")
        || text.contains("diagnostic::failure_value")
        || text.contains("receipt_from_control_graph")
        || text.contains("crate::json_boundary::write_json")
        || typed_record_projection_text(text)
}

fn typed_record_projection_text(text: &str) -> bool {
    if text.contains("-> Value")
        || text.contains("-> Result<Value")
        || text.contains("-> Option<Value")
        || text.contains("Value) -> Value")
    {
        return false;
    }
    let returns_typed_record = text.contains("-> String")
        || text.contains("-> &str")
        || text.contains("-> bool")
        || text.contains("-> Option<")
        || text.contains("-> Vec<")
        || (text.contains("-> BTreeMap<")
            && !text.contains("BTreeMap<String, Value")
            && !text.contains("HashMap<String, Value"))
        || text.contains("-> crate::cli::observe::telemetry::RuntimeTelemetry")
        || (text.contains("struct ") && text.contains("-> "));
    returns_typed_record
        && (text.contains("serde_json::Value")
            || text.contains("use serde_json::Value")
            || text.contains(": &Value")
            || text.contains(": &[Value]"))
}

fn parser_boundary_text(text: &str) -> bool {
    reads_structured_input(text)
        || schema_catalog_boundary(text)
        || typed_json_field_parser(text)
        || validator_artifact_parser(text)
}

fn reads_structured_input(text: &str) -> bool {
    text.contains("json_boundary::read_json")
        || text.contains("crate::json_boundary::read_json")
        || text.contains("serde_json::from_str")
        || text.contains("serde_json::from_value")
        || text.contains("serde_json::from_slice")
}

fn schema_catalog_boundary(text: &str) -> bool {
    text.contains("schema_catalog::load")
}

fn typed_json_field_parser(text: &str) -> bool {
    typed_failure_boundary_text(text)
        && (text.contains(".as_object()")
            || text.contains(".as_array()")
            || text.contains(".get(")
            || text.contains(".pointer(")
            || text.contains("Value::as_")
            || text.contains("str_field("))
}

fn validator_artifact_parser(text: &str) -> bool {
    text.contains("validator_artifacts: &[Value]") && typed_failure_boundary_text(text)
}

fn typed_failure_boundary_text(text: &str) -> bool {
    let returns_typed_failure = text.contains("-> Vec<String>")
        || text.contains("-> Result<")
        || text.contains("out: &mut Vec<")
        || text.contains("failures: &mut Vec<")
        || text.contains("type Failures = BTreeMap<String, Vec<String>>")
        || text.contains("Vec<Failure>")
        || text.contains("Vec<ResourcePurposeFailure>");
    returns_typed_failure
        && (text.contains("format!(\"")
            || text.contains("Failure::new")
            || text.contains("out.push(")
            || text.contains("Err("))
}

fn allowed_direct_receipt_path(rel: &str) -> bool {
    matches!(
        rel,
        "validator/src/cli/standards/gardener.rs"
            | "validator/src/cli/session.rs"
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
