use serde_json::json;
use std::{fs, path::Path};

pub(in crate::self_tests::cli::observe::query) fn write_observable_inventory(root: &Path) {
    let mut rows = serde_json::Map::new();
    let mut surface_rows = serde_json::Map::new();
    let commands = crate::audit::observability::required_commands()
        .iter()
        .map(|command| {
            super::board_rows::insert_command(&mut rows, command);
            json!(command)
        })
        .collect::<Vec<_>>();
    let surfaces = crate::audit::observability::required_surfaces()
        .iter()
        .map(|surface| {
            super::board_rows::insert_surface(&mut surface_rows, surface);
            json!(surface)
        })
        .collect::<Vec<_>>();
    let mut loop_rows = serde_json::Map::new();
    for stage in crate::audit::observability::required_loop_stages() {
        super::board_rows::insert_operating(&mut loop_rows, "loop", stage);
    }
    let mut signal_rows = serde_json::Map::new();
    for signal in crate::audit::observability::required_signal_classes() {
        super::board_rows::insert_operating(&mut signal_rows, "signal", signal);
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
            "research_inputs": super::research_inputs::inputs()
        },
        "observability_control_board": observable_control_board(),
        "row_requirements": row_requirements(),
        "command_observability_inventory": rows,
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
    .expect("write observable inventory");
    super::receipts::write_command_roundtrip_receipts(root);
}

fn observable_control_board() -> serde_json::Value {
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
        "status": "observable",
        "families": families,
        "claim_impact": "supports_observability_gate_only_when_every_inventory_row_is_command_observable_same_candidate"
    })
}

fn insert_counts(families: &mut serde_json::Map<String, serde_json::Value>, key: &str, len: usize) {
    families.insert(
        key.to_string(),
        json!({
            "total": len,
            "observable": len,
            "partially_observable": 0,
            "unobservable": 0
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
            super::board_rows::insert_dimension(&mut rows, board_key, id);
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
        "same_candidate_query_roundtrip": true,
        "validator_enforced": true,
        "owner_surface_tracking": true,
        "next_unobservable_surface_tracking": true,
        "observability_control_board": true,
        "red_fixture_proof": true,
        "green_fixture_proof": true,
        "tamper_fixture_proof": true,
        "explicit_instrumentation_fields": true
    })
}
