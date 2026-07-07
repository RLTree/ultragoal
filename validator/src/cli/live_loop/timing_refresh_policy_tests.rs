use super::changed_inputs::ChangedInputs;
use super::node_timing_refresh as subject;
use super::{LiveLoopAction, LiveLoopCommand, surfaces};
use serde_json::json;
use std::path::Path;

#[test]
fn node_timing_refresh_requires_verified_local_cache_mode() {
    let command = command("none");
    let blocker = json!({
        "id": "fmt_check",
        "failure_class": "live_loop_high_frequency_measurement_missing"
    });
    let inputs = changed_inputs_for(&["validator/src/cli/live_loop/mod.rs"]);
    let refresh = subject::refresh_hot_repair_timing(Path::new("."), &command, &blocker, &inputs)
        .expect("refresh policy check");
    assert!(refresh.is_empty());
}

#[test]
fn node_timing_refresh_ignores_non_timing_failure_classes() {
    let command = command("verified-local");
    let blocker = json!({
        "id": "fmt_check",
        "failure_class": "canonical_full_command_failed"
    });
    let inputs = changed_inputs_for(&["validator/src/cli/live_loop/mod.rs"]);
    let refresh = subject::refresh_hot_repair_timing(Path::new("."), &command, &blocker, &inputs)
        .expect("refresh policy check");
    assert!(refresh.is_empty());
}

#[test]
fn refresh_policy_preserves_validation_rows_when_observability_is_partial() {
    assert!(!subject::refreshable_failure_class(
        "live_loop_telemetry_reconciliation_missing"
    ));
}

#[test]
fn refresh_policy_bootstraps_routine_hot_nodes_without_boundary_proof_nodes() {
    let blocker = json!({
        "id": "schema_validation",
        "failure_class": "live_loop_high_frequency_measurement_missing"
    });
    let schema = surfaces::surface_by_id("schema_validation").expect("schema surface");
    let coverage = surfaces::surface_by_id("coverage_full_script").expect("coverage boundary");
    let inputs = changed_inputs_for(&["schemas/live-loop.schema.json"]);

    assert!(subject::should_refresh_surface(&schema, &blocker, &inputs));
    assert!(!coverage.high_frequency);
    assert!(!subject::should_refresh_surface(
        &coverage, &blocker, &inputs
    ));
}

#[test]
fn refresh_policy_does_not_remeasure_observability_partial_validation_rows() {
    let blocker = json!({
        "id": "schema_validation",
        "failure_class": "live_loop_telemetry_reconciliation_missing"
    });
    let schema = surfaces::surface_by_id("schema_validation").expect("schema surface");
    let namespace = surfaces::surface_by_id("namespace_check").expect("namespace surface");
    let inputs = changed_inputs_for(&["schemas/live-loop.schema.json"]);

    assert!(!subject::should_refresh_surface(&schema, &blocker, &inputs));
    assert!(!subject::should_refresh_surface(
        &namespace, &blocker, &inputs
    ));
}

#[test]
fn refresh_policy_does_not_remeasure_reusable_validation_for_speed_failure() {
    assert!(!subject::refreshable_failure_class(
        "live_loop_speedup_target_missed"
    ));
}

#[test]
fn timing_refresh_returns_no_work_when_refreshable_blocker_has_no_hot_surface() {
    let command = command("verified-local");
    let blocker = json!({
        "id": "unknown-node",
        "failure_class": "live_loop_high_frequency_measurement_missing"
    });
    let inputs = changed_inputs_for(&[]);

    let refresh = subject::refresh_hot_repair_timing(Path::new("."), &command, &blocker, &inputs)
        .expect("refreshable blocker with no mapped hot surface");

    assert!(refresh.is_empty());
}

#[test]
fn refresh_policy_selects_changed_input_affected_hot_surfaces() {
    let blocker = json!({
        "id": "fmt_check",
        "failure_class": "live_loop_high_frequency_measurement_missing"
    });
    let inputs = changed_inputs_for(&["schemas/live-loop.schema.json"]);
    let ids: Vec<&str> = subject::refresh_surfaces(&blocker, &inputs)
        .iter()
        .map(|surface| surface.id)
        .collect();
    assert_eq!(ids, ["schema_validation", "package_inventory"]);
}

#[test]
fn refresh_policy_falls_back_to_blocker_node_when_worktree_has_no_changed_hot_inputs() {
    let blocker = json!({
        "id": "fmt_check",
        "failure_class": "live_loop_high_frequency_measurement_missing"
    });
    let inputs = changed_inputs_for(&[]);
    let fmt = surfaces::surface_by_id("fmt_check").expect("fmt surface");
    let schema = surfaces::surface_by_id("schema_validation").expect("schema surface");

    assert!(inputs.affected_high_frequency_surfaces().is_empty());
    assert!(subject::should_refresh_surface(&fmt, &blocker, &inputs));
    assert!(!subject::should_refresh_surface(&schema, &blocker, &inputs));
    let ids: Vec<&str> = subject::refresh_surfaces(&blocker, &inputs)
        .iter()
        .map(|surface| surface.id)
        .collect();
    assert_eq!(ids, ["fmt_check"]);
}

#[test]
fn refresh_policy_returns_no_surfaces_for_unknown_or_boundary_blockers() {
    let inputs = changed_inputs_for(&[]);
    let unknown = json!({
        "id": "unknown-node",
        "failure_class": "live_loop_high_frequency_measurement_missing"
    });
    let coverage = json!({
        "id": "coverage_full_script",
        "failure_class": "live_loop_high_frequency_measurement_missing"
    });

    assert!(subject::refresh_surfaces(&unknown, &inputs).is_empty());
    assert!(subject::refresh_surfaces(&coverage, &inputs).is_empty());
}

fn changed_inputs_for(paths: &[&str]) -> ChangedInputs {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("loop-refresh-policy");
    std::fs::create_dir_all(&root).expect("root");
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&root)
        .output()
        .expect("git init");
    for path in paths {
        let full_path = root.join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(full_path, "changed\n").expect("changed file");
        std::process::Command::new("git")
            .args(["add", path])
            .current_dir(&root)
            .output()
            .expect("git add");
    }
    let inputs = ChangedInputs::collect(&root, "sha256:candidate", "hot", "verified-local");
    std::fs::remove_dir_all(root).expect("cleanup");
    inputs
}

fn command(cache_mode: &str) -> LiveLoopCommand {
    LiveLoopCommand {
        action: LiveLoopAction::Run,
        tier: "hot".to_string(),
        cache_mode: cache_mode.to_string(),
        jobs: Some(2),
        receipt: "validation_artifacts/observability/loop-test.json".into(),
        node_id: None,
        measure_all: false,
    }
}
