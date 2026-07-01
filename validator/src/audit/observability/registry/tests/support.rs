use serde_json::{Map, Value, json};
use std::{fs, path::Path};

mod rows;

pub(super) fn write_registry_root(root: &Path, inventory: Value) {
    fs::create_dir_all(root.join("templates/agent-standards")).expect("standards");
    fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory");
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::create_dir_all(root.join("fixtures/mandatory-law-surfaces/valid")).expect("fixtures");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    crate::json_boundary::write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[{"id": crate::audit::observability::LAW}]}),
    )
    .expect("standards json");
    crate::json_boundary::write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[{"obligation_id": crate::audit::observability::LAW}]}),
    )
    .expect("obligation json");
    crate::json_boundary::write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[{"id": crate::audit::observability::LAW}]}),
    )
    .expect("trace json");
    write_inventory(root, inventory);
    super::receipts::write_fitting_receipts(root);
}

pub(super) fn write_inventory(root: &Path, inventory: Value) {
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &inventory,
    )
    .expect("inventory json");
}

pub(super) fn write_valid_fixture(root: &Path) {
    fs::write(
        root.join(format!(
            "fixtures/mandatory-law-surfaces/valid/{}.json",
            crate::audit::observability::LAW
        )),
        "{}",
    )
    .expect("valid fixture");
}

pub(super) fn fitted_inventory() -> Value {
    let mut rows = Map::new();
    for command in super::super::fitting::REQUIRED_COMMANDS {
        rows.insert((*command).to_string(), rows::fitted_row(command));
    }
    let mut surface_rows = Map::new();
    for surface in super::super::surfaces::REQUIRED_SURFACES {
        surface_rows.insert((*surface).to_string(), rows::fitted_surface_row(surface));
    }
    let mut loop_rows = Map::new();
    for stage in super::super::operating::REQUIRED_LOOP_STAGES {
        loop_rows.insert(
            (*stage).to_string(),
            rows::fitted_operating_row("loop", stage),
        );
    }
    let mut signal_rows = Map::new();
    for signal in super::super::operating::REQUIRED_SIGNAL_CLASSES {
        signal_rows.insert(
            (*signal).to_string(),
            rows::fitted_operating_row("signal", signal),
        );
    }
    let mut inventory = json!({
        "commands": super::super::fitting::REQUIRED_COMMANDS,
        "surfaces": super::super::surfaces::REQUIRED_SURFACES,
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
            "research_inputs": super::super::research_inputs::fixture_inputs()
        },
        "fitting_control_board": fitted_control_board(),
        "row_requirements": {
            "log_instrumentation": true,
            "metric_instrumentation": true,
            "trace_instrumentation": true,
            "pass_output_contract": true,
            "fail_output_contract": true,
            "receipt_observability_binding": true,
            "focused_tests": true,
            "red_fixture_proof": true,
            "green_fixture_proof": true,
            "tamper_fixture_proof": true,
            "claim_impact_mapping": true,
            "same_candidate_query_proof": true,
            "explicit_instrumentation_fields": true,
            "validator_enforced": true,
            "owner_surface_tracking": true,
            "next_unfitted_surface_tracking": true,
            "fitting_control_board": true
        },
        "fitting_inventory": rows,
        "surface_inventory": surface_rows,
        "operating_loop_inventory": loop_rows,
        "signal_inventory": signal_rows
    });
    super::dimension_support::insert_dimension_inventory(&mut inventory);
    inventory
}

fn fitted_control_board() -> Value {
    let mut families = Map::new();
    insert_counts(
        &mut families,
        "commands",
        super::super::fitting::REQUIRED_COMMANDS.len(),
    );
    insert_counts(
        &mut families,
        "surfaces",
        super::super::surfaces::REQUIRED_SURFACES.len(),
    );
    insert_counts(
        &mut families,
        "operating_loop",
        super::super::operating::REQUIRED_LOOP_STAGES.len(),
    );
    insert_counts(
        &mut families,
        "signals",
        super::super::operating::REQUIRED_SIGNAL_CLASSES.len(),
    );
    super::dimension_support::insert_dimension_counts(&mut families);
    json!({
        "status": "fitted",
        "families": families,
        "claim_impact": "supports_gate_92_only_when_every_inventory_row_is_fitted_same_candidate"
    })
}

pub(super) fn insert_counts(families: &mut Map<String, Value>, key: &str, len: usize) {
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
