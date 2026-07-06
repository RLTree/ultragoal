use super::{first_blocker, required_high_frequency_validation_ids};
use crate::scheduler::TaskClass;
use serde_json::json;
use std::collections::BTreeMap;

mod timing;

#[test]
fn high_frequency_registry_covers_required_source_local_validation() {
    let ids = required_high_frequency_validation_ids();
    for required in [
        "scripts_check",
        "coverage_full_script",
        "coverage_fast_script",
        "coverage_prove",
        "source_audit",
        "red_fixture_report",
        "line_caps_check",
        "namespace_check",
        "schema_validation",
        "mandatory_law_validation",
        "source_obligations_check",
        "foundational_trace_check",
        "package_inventory",
        "focused_rust_tests",
        "fmt_check",
        "build_check",
        "touched_fixture_reports",
    ] {
        assert!(ids.contains(&required), "missing {required}: {ids:?}");
    }
}

#[test]
fn high_frequency_registry_classifies_authority_artifact_writers_as_serial() {
    for id in [
        "build_check",
        "focused_rust_tests",
        "line_caps_check",
        "namespace_check",
        "schema_validation",
        "package_inventory",
        "mandatory_law_validation",
        "source_obligations_check",
        "foundational_trace_check",
        "coverage_prove",
        "coverage_full_script",
        "coverage_fast_script",
        "source_audit",
        "red_fixture_report",
        "scripts_check",
        "touched_fixture_reports",
    ] {
        let surface = super::super::surfaces::surface_by_id(id).expect(id);
        assert_eq!(
            surface.execution_task_class,
            TaskClass::SharedAuthorityWriteSerial,
            "{id} writes authority artifacts and must not be executed by a parallel worker"
        );
        assert_ne!(surface.execution_serial_reason, "none", "{id}");
    }
}

#[test]
fn read_only_high_frequency_registry_entries_remain_parallel() {
    let fmt = super::super::surfaces::surface_by_id("fmt_check").expect("fmt");
    assert_eq!(fmt.execution_task_class, TaskClass::PureReadParallel);
    assert_eq!(fmt.execution_serial_reason, "none");
}

#[test]
fn live_loop_nodes_fail_closed_when_measurement_is_missing() {
    let nodes = vec![json!({
        "node_id": "coverage_full_script",
        "surface": "exact_coverage_script",
        "status": "blocked",
        "failure_class": "live_loop_high_frequency_measurement_missing",
        "why_failed": "high-frequency live-loop node lacks current full-command baseline and verified-local 20x timing proof",
        "where_failed": "loop.run.coverage_full_script.measurement",
        "next_repair": "measure canonical baseline",
        "narrow_rerun": "bash scripts/check-coverage-full .",
        "claim_impact": "blocks_live_loop_routine_repair_until_current_timing_proof"
    })];
    let blocker = first_blocker(&nodes).expect("node blocker");
    assert_eq!(blocker["id"], "coverage_full_script");
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
fn context_nodes_do_not_substitute_for_high_frequency_validation_speedproof() {
    let tasks = super::tasks(
        "sha256:candidate",
        "sha256:changed",
        "sha256:context",
        "hot",
        "verified-local",
        Some(1_000),
        &BTreeMap::new(),
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
