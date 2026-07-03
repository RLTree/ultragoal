use serde_json::Value;

const LAW: &str = "full-local-observability-stack-integration-non-opaque-failure";
const COMMAND_INVENTORY: &str = "docs/generated/observability/command-inventory.json";
const FINAL_PACKET_FIELD: &str = "observability_status";
const UPDATE_GOAL_BLOCKER: &str = "observability_product_closure_incomplete";

pub(super) fn required_field_failures(id: &str, row: &Value) -> Vec<String> {
    [
        "canonical_law_ids",
        "standards_row_ids",
        "source_obligation_ids",
        "foundational_trace_ids",
        "schemas",
        "validator_check_ids",
        "red_fixture_ids",
        "tamper_fixture_ids",
        "green_fixture_paths",
        "receipt_requirements",
        "package_inventory_paths",
        "setup_retrofit_outputs",
        "claim_guards",
        "final_packet_fields",
        "update_goal_blockers",
    ]
    .into_iter()
    .filter(|field| array(row, field).is_empty())
    .map(|field| format!("research_trace_missing_{field}:{id}"))
    .collect()
}

pub(super) fn row_failures(id: &str, row: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for field in [
        "canonical_law_ids",
        "standards_row_ids",
        "source_obligation_ids",
        "foundational_trace_ids",
        "validator_check_ids",
    ] {
        if !array(row, field).iter().any(|item| item == LAW) {
            out.push(format!(
                "research_trace_observability_binding_{field}_missing:{id}"
            ));
        }
    }
    if !array(row, "schemas")
        .iter()
        .any(|path| path.starts_with("schemas/observability-"))
    {
        out.push(format!(
            "research_trace_observability_binding_schema_missing:{id}"
        ));
    }
    if !array(row, "receipt_requirements")
        .iter()
        .any(|item| item.contains("observability") || item.contains("telemetry"))
    {
        out.push(format!(
            "research_trace_observability_binding_receipt_missing:{id}"
        ));
    }
    if !array(row, "package_inventory_paths")
        .iter()
        .any(|path| path == COMMAND_INVENTORY || path.starts_with("schemas/observability-"))
    {
        out.push(format!(
            "research_trace_observability_binding_package_path_missing:{id}"
        ));
    }
    if !array(row, "claim_guards")
        .iter()
        .any(|guard| guard.contains("observability"))
    {
        out.push(format!(
            "research_trace_observability_binding_claim_guard_missing:{id}"
        ));
    }
    if !array(row, "final_packet_fields")
        .iter()
        .any(|field| field == FINAL_PACKET_FIELD)
    {
        out.push(format!(
            "research_trace_observability_binding_final_packet_field_missing:{id}"
        ));
    }
    if !array(row, "update_goal_blockers")
        .iter()
        .any(|blocker| blocker == UPDATE_GOAL_BLOCKER)
    {
        out.push(format!(
            "research_trace_observability_binding_update_goal_blocker_missing:{id}"
        ));
    }
    out
}

fn array(row: &Value, key: &str) -> Vec<String> {
    row.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}
