use serde_json::{Map, Value, json};
use std::{fs, path::Path};

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
        rows.insert((*command).to_string(), fitted_row(command));
    }
    let mut surface_rows = Map::new();
    for surface in super::super::surfaces::REQUIRED_SURFACES {
        surface_rows.insert((*surface).to_string(), fitted_surface_row(surface));
    }
    let mut loop_rows = Map::new();
    for stage in super::super::operating::REQUIRED_LOOP_STAGES {
        loop_rows.insert((*stage).to_string(), fitted_operating_row("loop", stage));
    }
    let mut signal_rows = Map::new();
    for signal in super::super::operating::REQUIRED_SIGNAL_CLASSES {
        signal_rows.insert(
            (*signal).to_string(),
            fitted_operating_row("signal", signal),
        );
    }
    json!({
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
            }
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
            "claim_impact_mapping": true,
            "same_candidate_query_proof": true,
            "validator_enforced": true,
            "owner_surface_tracking": true,
            "next_unfitted_surface_tracking": true,
            "fitting_control_board": true
        },
        "fitting_inventory": rows,
        "surface_inventory": surface_rows,
        "operating_loop_inventory": loop_rows,
        "signal_inventory": signal_rows
    })
}

fn fitted_control_board() -> Value {
    json!({
        "status": "fitted",
        "families": {
            "commands": {
                "total": super::super::fitting::REQUIRED_COMMANDS.len(),
                "fitted": super::super::fitting::REQUIRED_COMMANDS.len(),
                "partially_fitted": 0,
                "unfitted": 0
            },
            "surfaces": {
                "total": super::super::surfaces::REQUIRED_SURFACES.len(),
                "fitted": super::super::surfaces::REQUIRED_SURFACES.len(),
                "partially_fitted": 0,
                "unfitted": 0
            },
            "operating_loop": {
                "total": super::super::operating::REQUIRED_LOOP_STAGES.len(),
                "fitted": super::super::operating::REQUIRED_LOOP_STAGES.len(),
                "partially_fitted": 0,
                "unfitted": 0
            },
            "signals": {
                "total": super::super::operating::REQUIRED_SIGNAL_CLASSES.len(),
                "fitted": super::super::operating::REQUIRED_SIGNAL_CLASSES.len(),
                "partially_fitted": 0,
                "unfitted": 0
            }
        },
        "claim_impact": "supports_gate_92_only_when_every_inventory_row_is_fitted_same_candidate"
    })
}

fn fitted_row(command: &str) -> Value {
    let slug = slug(command);
    json!({
        "fitting_status": "fitted",
        "fitted_surfaces": ["log", "metric", "trace", "stdout", "receipt"],
        "missing_surfaces": [],
        "validator_check_id": crate::audit::observability::LAW,
        "focused_tests": ["observability_registry_accepts_fully_fitted_inventory"],
        "receipt_paths": [format!("validation_artifacts/observability/fitting/{slug}.json")],
        "live_query_proof_paths": [
            format!("validation_artifacts/observability/fitting/{slug}-logs.json"),
            format!("validation_artifacts/observability/fitting/{slug}-metrics.json"),
            format!("validation_artifacts/observability/fitting/{slug}-traces.json")
        ],
        "current_owner_surface": format!("command:{command}"),
        "next_unfitted_surface": "none",
        "claim_impact": "supports_gate_92_when_same_candidate"
    })
}

fn fitted_surface_row(surface: &str) -> Value {
    let slug = slug(surface);
    json!({
        "fitting_status": "fitted",
        "fitted_surfaces": ["log", "metric", "trace", "stdout", "receipt"],
        "missing_surfaces": [],
        "operation": surface_operation(surface),
        "validator_check_id": crate::audit::observability::LAW,
        "focused_tests": ["observability_registry_accepts_fully_fitted_inventory"],
        "receipt_paths": [format!("validation_artifacts/observability/fitting/surface-{slug}.json")],
        "live_query_proof_paths": [
            format!("validation_artifacts/observability/fitting/surface-{slug}-logs.json"),
            format!("validation_artifacts/observability/fitting/surface-{slug}-metrics.json"),
            format!("validation_artifacts/observability/fitting/surface-{slug}-traces.json")
        ],
        "current_owner_surface": format!("surface:{surface}"),
        "next_unfitted_surface": "none",
        "claim_impact": "supports_gate_92_when_same_candidate"
    })
}

fn fitted_operating_row(kind: &str, name: &str) -> Value {
    let slug = slug(name);
    json!({
        "fitting_status": "fitted",
        "fitted_surfaces": ["log", "metric", "trace", "stdout", "receipt"],
        "missing_surfaces": [],
        "operation": format!("observability.{kind}.{slug}"),
        "validator_check_id": crate::audit::observability::LAW,
        "focused_tests": ["observability_registry_accepts_fully_fitted_inventory"],
        "receipt_paths": [format!("validation_artifacts/observability/fitting/{kind}-{slug}.json")],
        "live_query_proof_paths": [
            format!("validation_artifacts/observability/fitting/{kind}-{slug}-logs.json"),
            format!("validation_artifacts/observability/fitting/{kind}-{slug}-metrics.json"),
            format!("validation_artifacts/observability/fitting/{kind}-{slug}-traces.json")
        ],
        "current_owner_surface": format!("{kind}:{name}"),
        "next_unfitted_surface": "none",
        "claim_impact": "supports_gate_92_when_same_candidate"
    })
}

fn slug(command: &str) -> String {
    command.replace(' ', "-")
}

fn surface_operation(surface: &str) -> String {
    format!("surface.{}", surface.replace(' ', "."))
}
