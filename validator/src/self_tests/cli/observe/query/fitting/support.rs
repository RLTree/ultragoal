use serde_json::json;
use std::{fs, path::Path};

pub(in crate::self_tests::cli::observe::query) fn write_fitted_inventory(root: &Path) {
    let mut rows = serde_json::Map::new();
    let mut surface_rows = serde_json::Map::new();
    let commands = crate::audit::observability::required_commands()
        .iter()
        .map(|command| {
            insert_command_row(&mut rows, command);
            json!(command)
        })
        .collect::<Vec<_>>();
    let surfaces = crate::audit::observability::required_surfaces()
        .iter()
        .map(|surface| {
            insert_surface_row(&mut surface_rows, surface);
            json!(surface)
        })
        .collect::<Vec<_>>();
    let mut loop_rows = serde_json::Map::new();
    for stage in crate::audit::observability::required_loop_stages() {
        insert_operating_row(&mut loop_rows, "loop", stage);
    }
    let mut signal_rows = serde_json::Map::new();
    for signal in crate::audit::observability::required_signal_classes() {
        insert_operating_row(&mut signal_rows, "signal", signal);
    }
    fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory parent");
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &json!({
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
                }
            },
            "fitting_control_board": fitted_control_board(),
            "row_requirements": row_requirements(),
            "fitting_inventory": rows,
            "surface_inventory": surface_rows,
            "operating_loop_inventory": loop_rows,
            "signal_inventory": signal_rows
        }),
    )
    .expect("write fitted inventory");
    super::receipts::write_fitting_receipts(root);
}

fn fitted_control_board() -> serde_json::Value {
    json!({
        "status": "fitted",
        "families": {
            "commands": {
                "total": crate::audit::observability::required_commands().len(),
                "fitted": crate::audit::observability::required_commands().len(),
                "partially_fitted": 0,
                "unfitted": 0
            },
            "surfaces": {
                "total": crate::audit::observability::required_surfaces().len(),
                "fitted": crate::audit::observability::required_surfaces().len(),
                "partially_fitted": 0,
                "unfitted": 0
            },
            "operating_loop": {
                "total": crate::audit::observability::required_loop_stages().len(),
                "fitted": crate::audit::observability::required_loop_stages().len(),
                "partially_fitted": 0,
                "unfitted": 0
            },
            "signals": {
                "total": crate::audit::observability::required_signal_classes().len(),
                "fitted": crate::audit::observability::required_signal_classes().len(),
                "partially_fitted": 0,
                "unfitted": 0
            }
        },
        "claim_impact": "supports_gate_92_only_when_every_inventory_row_is_fitted_same_candidate"
    })
}

fn insert_command_row(rows: &mut serde_json::Map<String, serde_json::Value>, command: &str) {
    let slug = slug(command);
    rows.insert(
        command.to_string(),
        json!({
            "fitting_status": "fitted",
            "fitted_surfaces": ["log", "metric", "trace", "receipt", "query"],
            "missing_surfaces": [],
            "validator_check_id": "full-local-observability-stack-integration-non-opaque-failure",
            "focused_tests": ["observe_green_prove_and_query_helpers_are_typed"],
            "receipt_paths": [format!("validation_artifacts/observability/fitting/{slug}.json")],
            "live_query_proof_paths": [
                format!("validation_artifacts/observability/fitting/{slug}-logs.json"),
                format!("validation_artifacts/observability/fitting/{slug}-metrics.json"),
                format!("validation_artifacts/observability/fitting/{slug}-traces.json")
            ],
            "current_owner_surface": format!("command:{command}"),
            "next_unfitted_surface": "none",
            "claim_impact": "test fixture supports observe prove green path only"
        }),
    );
}

fn insert_surface_row(rows: &mut serde_json::Map<String, serde_json::Value>, surface: &str) {
    let slug = slug(surface);
    rows.insert(
        surface.to_string(),
        json!({
            "fitting_status": "fitted",
            "fitted_surfaces": ["log", "metric", "trace", "receipt", "query"],
            "missing_surfaces": [],
            "operation": surface_operation(surface),
            "validator_check_id": "full-local-observability-stack-integration-non-opaque-failure",
            "focused_tests": ["observe_green_prove_and_query_helpers_are_typed"],
            "receipt_paths": [format!("validation_artifacts/observability/fitting/surface-{slug}.json")],
            "live_query_proof_paths": [
                format!("validation_artifacts/observability/fitting/surface-{slug}-logs.json"),
                format!("validation_artifacts/observability/fitting/surface-{slug}-metrics.json"),
                format!("validation_artifacts/observability/fitting/surface-{slug}-traces.json")
            ],
            "current_owner_surface": format!("surface:{surface}"),
            "next_unfitted_surface": "none",
            "claim_impact": "test fixture supports observe prove green path only"
        }),
    );
}

fn insert_operating_row(
    rows: &mut serde_json::Map<String, serde_json::Value>,
    kind: &str,
    name: &str,
) {
    let slug = slug(name);
    rows.insert(
        name.to_string(),
        json!({
            "fitting_status": "fitted",
            "fitted_surfaces": ["log", "metric", "trace", "receipt", "query"],
            "missing_surfaces": [],
            "operation": format!("observability.{kind}.{slug}"),
            "validator_check_id": "full-local-observability-stack-integration-non-opaque-failure",
            "focused_tests": ["observe_green_prove_and_query_helpers_are_typed"],
            "receipt_paths": [format!("validation_artifacts/observability/fitting/{kind}-{slug}.json")],
            "live_query_proof_paths": [
                format!("validation_artifacts/observability/fitting/{kind}-{slug}-logs.json"),
                format!("validation_artifacts/observability/fitting/{kind}-{slug}-metrics.json"),
                format!("validation_artifacts/observability/fitting/{kind}-{slug}-traces.json")
            ],
            "current_owner_surface": format!("{kind}:{name}"),
            "next_unfitted_surface": "none",
            "claim_impact": "test fixture supports observe prove green path only"
        }),
    );
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
        "fitting_control_board": true
    })
}

fn slug(command: &str) -> String {
    command.replace(' ', "-")
}

fn surface_operation(surface: &str) -> String {
    format!("surface.{}", surface.replace(' ', "."))
}
