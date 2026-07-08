mod classifiers;
mod live_loop_projection_catalog;
mod markers;
mod projection;
mod projection_catalog;

use classifiers::{
    typed_cli_command_boundary_text, typed_law_check_boundary_text, typed_path_boundary_text,
};
use markers::{RawAuthorityMarker, raw_authority_markers};

pub(super) fn failures_for_text(rel: &str, text: &str) -> Vec<String> {
    raw_authority_markers(text)
        .into_iter()
        .filter(|marker| raw_authority_class(rel, text, marker).is_none())
        .map(|marker| {
            format!(
                "raw_downstream_authority_unclassified:path={rel};raw_authority={};classification_required=parser_boundary|projection|fixture_catalog_materialization|catalog_materialization;repair=route_raw_input_through_typed_record_or_typed_failure_before_law_execution",
                marker.as_str()
            )
        })
        .collect()
}

fn raw_authority_class(rel: &str, text: &str, marker: &RawAuthorityMarker) -> Option<&'static str> {
    if rel.contains("/self_tests/") || rel.contains("/tests/") || rel.ends_with("/tests.rs") {
        return Some("fixture_catalog_materialization");
    }
    if rel.contains("/test_rows/") {
        return Some("fixture_catalog_materialization");
    }
    if fixture_or_catalog_path(rel) {
        return Some("fixture_catalog_materialization");
    }
    if authority_surface_inventory_path(rel) {
        return Some("catalog_materialization");
    }
    if projection::boundary_text(rel, text, marker) {
        return Some("projection");
    }
    if parser_boundary_text(rel, text, marker) {
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
        || rel == "validator/src/audit/law/authority_surfaces/package_surfaces/row.rs"
}

fn parser_boundary_text(rel: &str, text: &str, marker: &RawAuthorityMarker) -> bool {
    parser_boundary_path(rel)
        || match marker {
            RawAuthorityMarker::RawPath => {
                typed_cli_command_boundary_text(rel, text)
                    || typed_path_boundary_text(rel, text)
                    || reads_structured_input(text)
                    || (contains_json_value_binding(text) && typed_law_check_boundary_text(text))
            }
            RawAuthorityMarker::RawMap => {
                reads_structured_input(text) || typed_law_check_boundary_text(text)
            }
            RawAuthorityMarker::RawJson
            | RawAuthorityMarker::RawString
            | RawAuthorityMarker::RawObservation => {
                reads_structured_input(text)
                    || typed_law_check_boundary_text(text)
                    || schema_catalog_boundary(text)
                    || typed_json_field_parser(text)
                    || validator_artifact_parser(text)
            }
        }
}

fn parser_boundary_path(rel: &str) -> bool {
    rel.ends_with("json_boundary.rs")
        || rel.contains("/authority_surfaces/source/")
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
        || text.contains("Vec<ResourcePurposeFailure>")
        || text.contains("Vec<SkillLinkFailure>");
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
