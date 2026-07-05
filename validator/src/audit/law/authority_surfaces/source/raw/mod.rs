mod classifiers;
mod markers;
mod projection;

use classifiers::{typed_cli_command_boundary_text, typed_law_check_boundary_text};
use markers::raw_authority_marker;

pub(super) fn failures_for_text(rel: &str, text: &str) -> Vec<String> {
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

fn raw_authority_class(rel: &str, text: &str) -> Option<&'static str> {
    if rel.contains("/self_tests/") || rel.contains("/tests/") || rel.ends_with("/tests.rs") {
        return Some("fixture_catalog_materialization");
    }
    if fixture_or_catalog_path(rel) {
        return Some("fixture_catalog_materialization");
    }
    if authority_surface_inventory_path(rel) {
        return Some("catalog_materialization");
    }
    if projection::boundary_text(rel, text) {
        return Some("projection");
    }
    if parser_boundary_text(rel, text) || typed_failure_boundary_text(text) {
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
        || rel == "validator/src/audit/namespace/source/rejection_ownership.rs"
        || rel.ends_with("schema_catalog.rs")
}

fn authority_surface_inventory_path(rel: &str) -> bool {
    rel.contains("/authority_surfaces/surface_inventory/discovered/")
}

fn parser_boundary_text(rel: &str, text: &str) -> bool {
    parser_boundary_path(rel)
        || reads_structured_input(text)
        || typed_cli_command_boundary_text(rel, text)
        || typed_law_check_boundary_text(text)
        || schema_catalog_boundary(text)
        || typed_json_field_parser(text)
        || validator_artifact_parser(text)
}

fn parser_boundary_path(rel: &str) -> bool {
    rel.ends_with("json_boundary.rs")
        || rel.contains("/authority_surfaces/source/raw")
        || rel.starts_with("validator/src/schema_catalog/")
        || rel == "validator/src/schema_catalog/mod.rs"
        || rel.contains("/schema/")
        || rel.contains("/parser/")
        || rel.contains("/parse")
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
        || text.contains("-> Option<String>")
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
            || text.contains("Err(")
            || text.contains("Some(format!(")
            || text.contains("Some(\""))
}

fn contains_json_value_binding(text: &str) -> bool {
    text.contains("serde_json::Value")
        || text.contains("use serde_json::Value")
        || text.contains("serde_json::{Value")
        || (text.contains("serde_json::{") && text.contains("Value"))
}
