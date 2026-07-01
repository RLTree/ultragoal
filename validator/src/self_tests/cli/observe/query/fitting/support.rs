use serde_json::json;
use std::{fs, path::Path};

mod research;
mod rows;

pub(in crate::self_tests::cli::observe::query) fn write_fitted_inventory(root: &Path) {
    let mut rows = serde_json::Map::new();
    let mut surface_rows = serde_json::Map::new();
    let commands = crate::audit::observability::required_commands()
        .iter()
        .map(|command| {
            rows::insert_command(&mut rows, command);
            json!(command)
        })
        .collect::<Vec<_>>();
    let surfaces = crate::audit::observability::required_surfaces()
        .iter()
        .map(|surface| {
            rows::insert_surface(&mut surface_rows, surface);
            json!(surface)
        })
        .collect::<Vec<_>>();
    let mut loop_rows = serde_json::Map::new();
    for stage in crate::audit::observability::required_loop_stages() {
        rows::insert_operating(&mut loop_rows, "loop", stage);
    }
    let mut signal_rows = serde_json::Map::new();
    for signal in crate::audit::observability::required_signal_classes() {
        rows::insert_operating(&mut signal_rows, "signal", signal);
    }
    let mut inventory = json!({
        "commands": commands,
        "surfaces": surfaces,
        "operating_loop": {
            "doctrine": {
                "agent_legibility_local_stack": true,
                "repair_validate_loop": true,
                "trace_whole_workflow": true,
                "freshness_prevents_false_conclusions": true,
                "four_golden_signals_for_cli": true,
                "wide_structured_events_with_bounded_context": true,
                "semantic_naming_across_telemetry": true
            },
            "research_inputs": research::inputs()
        },
        "fitting_control_board": fitted_control_board(),
        "row_requirements": row_requirements(),
        "fitting_inventory": rows,
        "surface_inventory": surface_rows,
        "operating_loop_inventory": loop_rows,
        "signal_inventory": signal_rows
    });
    insert_dimension_inventory(&mut inventory);
    fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory parent");
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &inventory,
    )
    .expect("write fitted inventory");
    super::receipts::write_fitting_receipts(root);
}

fn fitted_control_board() -> serde_json::Value {
    let mut families = serde_json::Map::new();
    insert_counts(
        &mut families,
        "commands",
        crate::audit::observability::required_commands().len(),
    );
    insert_counts(
        &mut families,
        "surfaces",
        crate::audit::observability::required_surfaces().len(),
    );
    insert_counts(
        &mut families,
        "operating_loop",
        crate::audit::observability::required_loop_stages().len(),
    );
    insert_counts(
        &mut families,
        "signals",
        crate::audit::observability::required_signal_classes().len(),
    );
    for (board_key, _, _, ids) in crate::audit::observability::required_dimension_families() {
        insert_counts(&mut families, board_key, ids.len());
    }
    json!({
        "status": "fitted",
        "families": families,
        "claim_impact": "supports_observability_gate_only_when_every_inventory_row_is_fitted_same_candidate"
    })
}

fn insert_counts(families: &mut serde_json::Map<String, serde_json::Value>, key: &str, len: usize) {
    families.insert(
        key.to_string(),
        json!({
            "total": len,
            "fitted": len,
            "partially_fitted": 0,
            "unfitted": 0
        }),
    );
}

fn insert_dimension_inventory(inventory: &mut serde_json::Value) {
    for (board_key, list_key, inventory_key, ids) in
        crate::audit::observability::required_dimension_families()
    {
        inventory[list_key] = json!(ids);
        let mut rows = serde_json::Map::new();
        for id in ids {
            rows::insert_dimension(&mut rows, board_key, id);
        }
        inventory[inventory_key] = serde_json::Value::Object(rows);
    }
}

fn row_requirements() -> serde_json::Value {
    json!({
        "log_instrumentation": true,
        "metric_instrumentation": true,
        "trace_instrumentation": true,
        "pass_output_contract": true,
        "fail_output_contract": true,
        "receipt_observability_binding": true,
        "focused_tests": true,
        "claim_impact_mapping": true,
        "same_candidate_query_proof": true,
        "validator_enforced": true,
        "owner_surface_tracking": true,
        "next_unfitted_surface_tracking": true,
        "fitting_control_board": true,
        "red_fixture_proof": true,
        "green_fixture_proof": true,
        "tamper_fixture_proof": true,
        "explicit_instrumentation_fields": true
    })
}
