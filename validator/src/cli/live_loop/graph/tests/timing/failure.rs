use super::rows::{mandatory_law_failure, node_timing, projected_node};

#[test]
fn live_loop_graph_projects_executed_work_failure_locations() {
    let cases = [
        (
            "verified_local_command_launch_failed",
            "verified_local_command_launch_failed",
            "verified_local",
            "could not launch",
            "verified_local_command_launch_failed",
        ),
        (
            "verified_local_command_failed",
            "verified_local_command_failed",
            "verified_local",
            "exited nonzero",
            "verified_local_command_failed",
        ),
        (
            "verified_local_proof_kind_invalid",
            "verified_local_proof_kind_invalid",
            "proof_kind",
            "proof-shaped output",
            "verified_local_proof_kind_invalid",
        ),
        (
            "verified_local_cache_equivalence_missing",
            "verified_local_cache_equivalence_missing",
            "cache_equivalence",
            "cache reuse without same-candidate equivalence",
            "verified_local_cache_equivalence_missing",
        ),
        (
            "verified_local_work_unit_missing",
            "verified_local_work_unit_missing",
            "work_unit",
            "no executed work unit",
            "verified_local_work_unit_missing",
        ),
        (
            "verified_local_equivalence_status_invalid",
            "verified_local_equivalence_status_invalid",
            "equivalence_status",
            "execution equivalence status",
            "verified_local_equivalence_status_invalid",
        ),
        (
            "verified_local_invalidation_proof_missing",
            "verified_local_invalidation_proof_missing",
            "invalidation_proof",
            "cache invalidation proof",
            "verified_local_invalidation_proof_missing",
        ),
        (
            "live_loop_telemetry_reconciliation_missing",
            "live_loop_telemetry_reconciliation_missing",
            "telemetry_reconciliation",
            "same-candidate telemetry reconciliation",
            "telemetry_reconciliation_missing",
        ),
        (
            "live_loop_speedup_target_missed",
            "live_loop_speedup_target_missed",
            "speedup",
            "20x speed target",
            "verified_local_20x_proof_failed",
        ),
        (
            "unexpected_measurement_failure",
            "live_loop_node_measurement_failed",
            "measurement",
            "current high-frequency live-loop node timing row is failed",
            "verified_local_measurement_failed",
        ),
    ];

    for (input_class, expected_class, where_suffix, why_fragment, speedup_state) in cases {
        let node = projected_node(node_timing(
            100_000,
            1,
            "fail".to_string(),
            input_class.to_string(),
        ));
        assert_eq!(node["status"], "blocked");
        assert_eq!(node["failure_class"], expected_class);
        assert_eq!(
            node["where_failed"],
            format!("loop.run.focused_rust_tests.{where_suffix}")
        );
        assert!(
            node["why_failed"]
                .as_str()
                .expect("why")
                .contains(why_fragment)
        );
        assert_eq!(node["speedup_measurement_state"], speedup_state);
    }
}

#[test]
fn live_loop_graph_blocks_pass_shaped_timing_that_misses_speed_target() {
    let node = projected_node(node_timing(
        1_000,
        100,
        "pass".to_string(),
        "none".to_string(),
    ));

    assert_eq!(node["status"], "blocked");
    assert_eq!(node["failure_class"], "live_loop_speedup_target_missed");
    assert_eq!(
        node["speedup_measurement_state"],
        "verified_local_20x_proof_failed"
    );
    assert_eq!(node["speedup_ratio"], 9);
}

#[test]
fn live_loop_graph_blocks_pass_shaped_timing_without_reconciliation() {
    let mut timing = node_timing(1_000, 1, "pass".to_string(), "none".to_string());
    timing.telemetry_reconciliation_status = "missing".to_string();

    let node = projected_node(timing);

    assert_eq!(node["status"], "blocked");
    assert_eq!(
        node["failure_class"],
        "live_loop_telemetry_reconciliation_missing"
    );
    assert_eq!(
        node["speedup_measurement_state"],
        "telemetry_reconciliation_missing"
    );
}

#[test]
fn live_loop_graph_uses_baseline_failure_details_without_duplicate_rerun() {
    let rerun = "target/debug/ultragoal --root . loop measure --node focused_rust_tests --tier hot";
    let mut timing = node_timing(
        100_000,
        1,
        "fail".to_string(),
        "canonical_full_command_failed".to_string(),
    );
    timing.where_failed = "none".to_string();
    timing.why_failed = "none".to_string();
    timing.next_repair = rerun.to_string();
    timing.baseline_failure = mandatory_law_failure();

    let node = projected_node(timing);
    assert_eq!(node["where_failed"], "mandatory-law.validation");
    assert!(
        node["why_failed"]
            .as_str()
            .expect("why")
            .contains("stale receipt")
    );
    assert_eq!(node["next_repair"], rerun);
}

#[test]
fn live_loop_graph_names_canonical_baseline_when_failure_has_no_detail_fields() {
    let node = projected_node(node_timing(
        100_000,
        1,
        "fail".to_string(),
        "canonical_full_command_failed".to_string(),
    ));

    assert_eq!(node["where_failed"], "loop.run.focused_rust_tests.baseline");
    assert!(
        node["next_repair"]
            .as_str()
            .expect("repair")
            .contains("cargo test --offline --lib --quiet")
    );
}

#[test]
fn live_loop_graph_names_launch_failure_without_exit_code_guessing() {
    let mut timing = node_timing(
        100_000,
        1,
        "fail".to_string(),
        "canonical_full_command_launch_failed".to_string(),
    );
    timing.baseline_launch_error = true;
    timing.baseline_exit_code = None;

    let node = projected_node(timing);
    let next_repair = node["next_repair"].as_str().expect("repair");
    assert_eq!(node["where_failed"], "loop.run.focused_rust_tests.baseline");
    assert!(next_repair.contains("launch successfully"));
    assert!(!next_repair.contains("exit code unknown"));
}
