use super::markers::RawAuthorityMarker;

pub(super) fn boundary_text(rel: &str, text: &str, marker: &RawAuthorityMarker) -> bool {
    match marker {
        RawAuthorityMarker::RawJson => {
            typed_record_projection_text(text)
                || classified_product_projection_boundary(rel, text)
                || product_map_projection_text(text)
        }
        RawAuthorityMarker::RawMap => {
            classified_product_projection_boundary(rel, text) || product_map_projection_text(text)
        }
        RawAuthorityMarker::RawPath => classified_product_projection_boundary(rel, text),
        RawAuthorityMarker::RawObservation | RawAuthorityMarker::RawString => false,
    }
}

fn classified_product_projection_boundary(rel: &str, text: &str) -> bool {
    let Some(required) = super::projection_catalog::required_projection_markers(rel) else {
        return false;
    };
    required.iter().all(|needle| text.contains(needle)) && projection_value_text(text)
}

fn product_map_projection_text(text: &str) -> bool {
    let owns_check_map = text.contains("checks: &mut serde_json::Map<String, Value>")
        || text.contains("let mut checks = serde_json::Map::new()")
        || text.contains("pub checks: serde_json::Map<String, Value>");
    owns_check_map
        && (text.contains("checks.insert(")
            || text.contains("Value::Object(input.checks)")
            || text.contains("json!({")
            || text.contains("target_receipt("))
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
        || text.contains("telemetry::query_result")
        || text.contains("PathBuf::from(format!(")
        || text.contains("CommandTelemetry")
        || text.contains("metric.to_value(")
        || text.contains("FnOnce() -> Value")
        || text.contains("diagnostic::failure_value")
        || text.contains("receipt_from_control_graph")
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
        && (super::contains_json_value_binding(text)
            || text.contains(": &Value")
            || text.contains(": &[Value]"))
}
