use serde_json::{Map, Value, json};

pub(super) fn insert_dimension_inventory(inventory: &mut Value) {
    for family in super::super::dimension_ids::inventory_families() {
        inventory[family.list_key] = json!(family.ids);
        let mut rows = Map::new();
        for id in family.ids {
            rows.insert(
                (*id).to_string(),
                fitted_dimension_row(family.board_key, id),
            );
        }
        inventory[family.inventory_key] = Value::Object(rows);
    }
}

pub(super) fn insert_dimension_counts(families: &mut Map<String, Value>) {
    for family in super::super::dimension_ids::inventory_families() {
        super::support::insert_counts(families, family.board_key, family.ids.len());
    }
}

fn fitted_dimension_row(kind: &str, name: &str) -> Value {
    let slug = slug(name);
    json!({
        "fitting_status": "fitted",
        "fitted_surfaces": [
            "log instrumentation",
            "metric instrumentation",
            "trace instrumentation",
            "pass stdout contract",
            "fail stdout contract",
            "receipt observability binding"
        ],
        "missing_surfaces": [],
        "operation": format!("observability.{kind}.{slug}"),
        "validator_check_id": crate::audit::observability::LAW,
        "log_instrumentation": "log instrumentation fitted",
        "metric_instrumentation": "metric instrumentation fitted",
        "trace_instrumentation": "trace instrumentation fitted",
        "pass_stdout_contract": "pass stdout contract fitted",
        "fail_stdout_contract": "fail stdout contract fitted",
        "receipt_observability_binding": "receipt observability binding fitted",
        "focused_tests": ["observability_registry_accepts_fully_fitted_inventory"],
        "receipt_paths": [format!("validation_artifacts/observability/command-roundtrip/{kind}-{slug}.json")],
        "live_query_proof_paths": [
            format!("validation_artifacts/observability/command-roundtrip/{kind}-{slug}-logs.json"),
            format!("validation_artifacts/observability/command-roundtrip/{kind}-{slug}-metrics.json"),
            format!("validation_artifacts/observability/command-roundtrip/{kind}-{slug}-traces.json")
        ],
        "same_candidate_query_proof_paths": [
            format!("validation_artifacts/observability/command-roundtrip/{kind}-{slug}-logs.json"),
            format!("validation_artifacts/observability/command-roundtrip/{kind}-{slug}-metrics.json"),
            format!("validation_artifacts/observability/command-roundtrip/{kind}-{slug}-traces.json")
        ],
        "red_fixtures": [format!("fixtures/red/observability/{kind}-{slug}-red.json")],
        "green_fixtures": [format!("fixtures/green/observability/{kind}-{slug}-green.json")],
        "tamper_fixtures": [format!("fixtures/tamper/observability/{kind}-{slug}-tamper.json")],
        "current_owner_surface": format!("{kind}:{name}"),
        "next_unfitted_surface": "none",
        "claim_impact": "supports_observability_gate_when_same_candidate"
    })
}

fn slug(command: &str) -> String {
    command.replace(' ', "-")
}
