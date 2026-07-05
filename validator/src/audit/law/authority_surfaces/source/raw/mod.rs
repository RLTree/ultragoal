mod classifiers;
mod markers;

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
    if projection_boundary_text(rel, text) {
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
        || rel.ends_with("schema_catalog.rs")
}

fn authority_surface_inventory_path(rel: &str) -> bool {
    rel.contains("/authority_surfaces/surface_inventory/discovered/")
}

fn projection_boundary_text(rel: &str, text: &str) -> bool {
    typed_record_projection_text(text)
        || classified_product_projection_boundary(rel, text)
        || (projection_boundary_path(rel) && projection_value_text(text))
}

fn classified_product_projection_boundary(rel: &str, text: &str) -> bool {
    let required: &[&str] = match rel {
        "validator/src/cli/control/plane/mod.rs" => &[
            "ControlOperation",
            "receipt_from_control_graph",
            "registry::stdout::print",
        ],
        "validator/src/cli/control/plane/proof/mod.rs" => &[
            "ControlOperation",
            "diagnostic::failure_value",
            "diagnostic::notes",
        ],
        "validator/src/cli/control/plane/registry/capability/gap.rs" => &[
            "missing_capability_class",
            "affected_claim_ids",
            "current_claim_ceiling",
        ],
        "validator/src/cli/final_packet/proof/spans.rs" => {
            &["span_kind", "receipt_deref", "dereferenced_receipt_digest"]
        }
        "validator/src/cli/live_loop/context.rs" => {
            &["AuditContext", "changed_files_digest", "input_digest"]
        }
        "validator/src/cli/live_loop/graph.rs" => {
            &["LoopValidationSurface", "input_digest", "claim_impact"]
        }
        "validator/src/cli/observe/explain/summary.rs" => {
            &["ExplainContext", "smallest_repair", "query_evidence"]
        }
        "validator/src/cli/openai/config.rs" => &[
            "openai_config_redacted_resolution",
            "secret_material_serialized",
            "blocked_claims",
        ],
        "validator/src/audit/law/authority_surfaces/surface_inventory/mod.rs" => &[
            "AuthoritySurfaceInventoryRow",
            "harness-ultragoal.foundational-law-surface-inventory.v1",
            "surface_state",
        ],
        "validator/src/cli/product/cohesion.rs" => &[
            "product-cohesion",
            "source_local_product_cohesion_only",
            "target_repo::product::cohesion::check",
        ],
        _ => return false,
    };
    required.iter().all(|needle| text.contains(needle)) && projection_value_text(text)
}

fn projection_boundary_path(rel: &str) -> bool {
    [
        "/stdout",
        "/receipt",
        "/receipts",
        "/telemetry",
        "/query",
        "/snapshot",
        "/report",
        "/diagnostic",
        "/emit",
        "/output",
        "/current_state",
        "/archive",
        "/review/",
        "/package/digest.rs",
        "/semantic/receipt",
    ]
    .iter()
    .any(|needle| rel.contains(needle))
        || rel.ends_with("target_repo/receipt.rs")
}

fn projection_value_text(text: &str) -> bool {
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
        && (contains_json_value_binding(text)
            || text.contains(": &Value")
            || text.contains(": &[Value]"))
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
