use super::{
    first_blocker, first_observability_blocker, first_product_blocker, first_speed_blocker,
};
use crate::cli::live_loop::changed_inputs::ChangedInputs;
use serde_json::json;
use std::collections::BTreeMap;

mod input_digest;
mod registry;
mod timing;

#[test]
fn validator_version_uses_stable_cli_authority_not_wrapper_binary() {
    let material = super::validator_authority_material();
    assert!(material.contains("authority=ultragoal-cli-control-plane"));
    assert!(material.contains("cli=ultragoal"));
    assert!(
        !material.contains("target/debug"),
        "validator authority must not depend on the invoked wrapper path"
    );
    assert_eq!(
        super::validator_version(),
        crate::digest::bytes(material.as_bytes())
    );
}

#[test]
fn live_loop_nodes_fail_closed_when_measurement_is_missing() {
    let nodes = vec![json!({
        "node_id": "schema_validation",
        "surface": "schema_catalog",
        "status": "blocked",
        "failure_class": "live_loop_high_frequency_measurement_missing",
        "why_failed": "high-frequency live-loop node lacks current full-command baseline and verified-local 20x timing proof",
        "where_failed": "loop.run.schema_validation.measurement",
        "next_repair": "measure canonical baseline",
        "narrow_rerun": "target/debug/ultragoal --root . schema validation --jobs 8",
        "claim_impact": "blocks_live_loop_routine_repair_until_current_timing_proof"
    })];
    let blocker = first_blocker(&nodes).expect("node blocker");
    assert_eq!(blocker["id"], "schema_validation");
    assert_eq!(
        blocker["failure_class"],
        "live_loop_high_frequency_measurement_missing"
    );
    assert_eq!(
        blocker["claim_impact"],
        "blocks_live_loop_routine_repair_until_current_timing_proof"
    );
}

#[test]
fn blocker_selection_prefers_product_failures_and_preserves_proof_blockers() {
    let nodes = vec![
        json!({
            "node_id": "fmt_check",
            "surface": "format_check",
            "status": "blocked",
            "validation_status": "pass",
            "observability_status": "pass",
            "speed_claim_status": "failed",
            "failure_class": "live_loop_speedup_target_missed",
            "why_failed": "format validation passed but speed target failed",
            "where_failed": "loop.run.fmt_check.speedup",
            "next_repair": "reuse or split format validation",
            "narrow_rerun": "cargo fmt --all --check",
            "claim_impact": "speed_claim_failed_validation_available"
        }),
        json!({
            "node_id": "build_check",
            "surface": "compile_check",
            "status": "blocked",
            "validation_status": "fail",
            "observability_status": "partial",
            "speed_claim_status": "withheld",
            "failure_class": "canonical_full_command_failed",
            "why_failed": "cargo build failed",
            "where_failed": "loop.run.build_check.baseline",
            "next_repair": "repair compile error",
            "narrow_rerun": "cargo build --offline --bin ultragoal --quiet",
            "claim_impact": "validation_failed"
        }),
        json!({
            "node_id": "observe_trace",
            "surface": "trace_query",
            "status": "partial",
            "validation_status": "pass",
            "observability_status": "partial",
            "speed_claim_status": "withheld",
            "failure_class": "live_loop_telemetry_reconciliation_missing",
            "why_failed": "trace query lagged",
            "where_failed": "loop.run.observe_trace.telemetry",
            "next_repair": "rerun trace query",
            "narrow_rerun": "ultragoal observe traces query --run-id run-x",
            "claim_impact": "observability_claim_withheld"
        }),
    ];

    assert_eq!(first_blocker(&nodes).expect("first")["id"], "build_check");
    assert_eq!(
        first_product_blocker(&nodes).expect("product")["id"],
        "build_check"
    );
    assert_eq!(
        first_observability_blocker(&nodes).expect("observability")["id"],
        "observe_trace"
    );
    assert_eq!(
        first_speed_blocker(&nodes).expect("speed")["id"],
        "fmt_check"
    );
}

#[test]
fn context_nodes_do_not_substitute_for_high_frequency_validation_speedproof() {
    let tasks = super::tasks(
        "sha256:candidate",
        "sha256:changed",
        "sha256:context",
        "hot",
        "verified-local",
        Some(1_000),
        &BTreeMap::new(),
        &ChangedInputs::for_tests("sha256:changed", "sha256:context"),
    );
    let first_node = tasks.into_iter().next().expect("package node")();
    assert_eq!(first_node["node_id"], "package_digest");
    assert_eq!(first_node["status"], "observed");
    assert_eq!(first_node["graph_task_class"], "pure_read_parallel");
    assert_eq!(
        first_node["execution_task_class"],
        "shared_authority_write_serial"
    );
    assert!(
        first_node["execution_serial_reason"]
            .as_str()
            .expect("serial reason")
            .contains("validation_artifacts")
    );
    assert_eq!(
        first_node["baseline_measurement_state"],
        "not_required_for_context_or_control_node"
    );
    assert_eq!(
        first_node["speedup_measurement_state"],
        "not_required_for_context_or_control_node"
    );
    assert_eq!(first_node["claim_status"], "observation_only");
    assert_eq!(
        first_node["claim_impact"],
        "observation_only_no_speed_readiness_release_completion_or_update_goal_claim"
    );
    assert!(
        first_blocker(&[first_node]).is_none(),
        "context observations must not become blockers or proof"
    );
}

#[test]
fn boundary_nodes_are_withheld_without_blocking_dirty_hot_loop_validation() {
    let surface = super::super::surfaces::surface_by_id("coverage_full_script")
        .expect("coverage full boundary surface");
    let node = super::surface_record(
        surface,
        "sha256:coverage-input",
        "hot",
        "verified-local",
        None,
        None,
    );

    assert_eq!(node["status"], "withheld");
    assert_eq!(
        node["failure_class"],
        "boundary_proof_withheld_from_hot_loop"
    );
    assert_eq!(
        node["hot_loop_policy"],
        super::super::surfaces::BOUNDARY_PROOF_POLICY
    );
    assert_eq!(node["claim_status"], "withheld_until_boundary");
    assert!(
        first_blocker(&[node]).is_none(),
        "boundary proof withholding must not erase useful hot-loop validation"
    );
}

#[test]
fn high_frequency_nodes_without_timing_rows_report_missing_measurement() {
    let surface = super::super::surfaces::LOOP_VALIDATION_SURFACES
        .iter()
        .find(|surface| surface.id == "fmt_check")
        .copied()
        .expect("fmt check surface");

    let missing_timing_node = super::surface_record(
        surface,
        "sha256:fmt-input",
        "hot",
        "verified-local",
        Some(1_000),
        None,
    );
    assert_eq!(missing_timing_node["status"], "blocked");
    assert_eq!(
        missing_timing_node["speedup_measurement_state"],
        "missing_verified_local_20x_proof"
    );
    assert_eq!(
        missing_timing_node["failure_class"],
        "live_loop_high_frequency_measurement_missing"
    );

    let other_missing_timing_node = super::surface_record(
        surface,
        "sha256:fmt-input",
        "hot",
        "verified-local",
        Some(1),
        None,
    );
    assert_eq!(other_missing_timing_node["status"], "blocked");
    assert_eq!(
        other_missing_timing_node["failure_class"],
        "live_loop_high_frequency_measurement_missing"
    );
    assert!(
        other_missing_timing_node["next_repair"]
            .as_str()
            .expect("repair text")
            .contains("measure canonical baseline")
    );
}
