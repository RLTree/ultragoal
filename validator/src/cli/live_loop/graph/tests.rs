use super::{first_blocker, required_high_frequency_validation_ids};
use serde_json::json;

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
    );
    let first_node = tasks.into_iter().next().expect("package node")();
    assert_eq!(first_node["node_id"], "package_digest");
    assert_eq!(first_node["status"], "pass");
    assert_eq!(
        first_node["baseline_measurement_state"],
        "not_required_for_context_or_control_node"
    );
    assert_eq!(
        first_node["speedup_measurement_state"],
        "not_required_for_context_or_control_node"
    );
}

#[test]
fn high_frequency_nodes_report_speedup_pass_and_miss_states() {
    let surface = super::super::surfaces::LOOP_VALIDATION_SURFACES
        .iter()
        .find(|surface| surface.id == "fmt_check")
        .copied()
        .expect("fmt check surface");

    let fast_node = super::surface_record(
        surface,
        "sha256:fmt-input",
        "hot",
        "verified-local",
        Some(1_000),
    );
    assert_eq!(fast_node["status"], "pass");
    assert_eq!(
        fast_node["speedup_measurement_state"],
        "verified_local_20x_proof_observed"
    );
    assert_eq!(
        fast_node["claim_impact"],
        "supports_source_local_live_loop_node_measurement_only"
    );

    let slow_node = super::surface_record(
        surface,
        "sha256:fmt-input",
        "hot",
        "verified-local",
        Some(1),
    );
    assert_eq!(slow_node["status"], "blocked");
    assert_eq!(
        slow_node["failure_class"],
        "live_loop_speedup_target_missed"
    );
    assert!(
        slow_node["next_repair"]
            .as_str()
            .expect("repair text")
            .contains("cargo fmt --all --check")
    );
}
