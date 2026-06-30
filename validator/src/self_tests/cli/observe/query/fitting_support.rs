use serde_json::json;
use std::{fs, path::Path};

pub(super) fn write_fitted_inventory(root: &Path) {
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
            "row_requirements": row_requirements(),
            "fitting_inventory": rows,
            "surface_inventory": surface_rows,
            "operating_loop_inventory": loop_rows,
            "signal_inventory": signal_rows
        }),
    )
    .expect("write fitted inventory");
    write_fitting_receipts(root);
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
            "claim_impact": "test fixture supports observe prove green path only"
        }),
    );
}

fn write_fitting_receipts(root: &Path) {
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let dir = root.join("validation_artifacts/observability/fitting");
    fs::create_dir_all(&dir).expect("fitting dir");
    for command in crate::audit::observability::required_commands() {
        let slug = slug(command);
        write_receipt_set(
            &dir,
            &candidate,
            &slug,
            &format!("run-{slug}"),
            &format!("corr-{slug}"),
            &command.replace(' ', "."),
        );
    }
    for surface in crate::audit::observability::required_surfaces() {
        let slug = slug(surface);
        write_receipt_set(
            &dir,
            &candidate,
            &format!("surface-{slug}"),
            &format!("run-surface-{slug}"),
            &format!("corr-surface-{slug}"),
            &surface_operation(surface),
        );
    }
    for stage in crate::audit::observability::required_loop_stages() {
        let slug = slug(stage);
        write_receipt_set(
            &dir,
            &candidate,
            &format!("loop-{slug}"),
            &format!("run-loop-{slug}"),
            &format!("corr-loop-{slug}"),
            &format!("observability.loop.{slug}"),
        );
    }
    for signal in crate::audit::observability::required_signal_classes() {
        let slug = slug(signal);
        write_receipt_set(
            &dir,
            &candidate,
            &format!("signal-{slug}"),
            &format!("run-signal-{slug}"),
            &format!("corr-signal-{slug}"),
            &format!("observability.signal.{slug}"),
        );
    }
}

fn write_receipt_set(
    dir: &Path,
    candidate: &str,
    slug: &str,
    run: &str,
    corr: &str,
    operation: &str,
) {
    crate::json_boundary::write_json(
        &dir.join(format!("{slug}.json")),
        &json!({
            "schema": crate::cli::observe::types::RECEIPT_SCHEMA,
            "status": "pass",
            "candidate_digest": candidate,
            "operation": operation,
            "run_id": run,
            "correlation_id": corr
        }),
    )
    .expect("receipt");
    for kind in ["logs", "metrics", "traces"] {
        crate::json_boundary::write_json(
            &dir.join(format!("{slug}-{kind}.json")),
            &json!({
                "schema": crate::cli::observe::types::QUERY_SCHEMA,
                "status": "pass",
                "candidate_digest": candidate,
                "run_id": run,
                "correlation_id": corr,
                "query_kind": kind,
                "rows": [{
                    "candidate_digest": candidate,
                    "operation": operation,
                    "correlation_id": corr
                }]
            }),
        )
        .expect("query receipt");
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
        "validator_enforced": true
    })
}

fn slug(command: &str) -> String {
    command.replace(' ', "-")
}

fn surface_operation(surface: &str) -> String {
    format!("surface.{}", surface.replace(' ', "."))
}
