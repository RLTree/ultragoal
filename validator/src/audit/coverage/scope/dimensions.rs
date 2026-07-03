use serde_json::Value;

const REQUIRED_DIMENSIONS: &[&str] = &["line", "branch", "function", "artifact", "ui_state"];

pub(crate) fn failures(value: &Value) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    for row in value
        .get("required_measured_dimensions_per_root")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for dim in row
            .get("dimensions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            seen.insert(dim.to_string());
        }
    }
    let mut out = Vec::new();
    for dim in REQUIRED_DIMENSIONS {
        if !seen.contains(*dim) {
            out.push(message_for_missing_dimension(dim).to_string());
        }
    }
    out
}

fn message_for_missing_dimension(dimension: &str) -> &str {
    match dimension {
        "ui_state" => "coverage_ui_state_missing_for_product_surface",
        "artifact" => "coverage_artifact_dimension_missing_for_generated_authority",
        _ => "coverage_behavior_dimension_missing",
    }
}
