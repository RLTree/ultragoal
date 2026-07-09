use super::{NODE_TIMING_REL, read_current};
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;

#[path = "observation_fixture.rs"]
mod observation_fixture;
#[path = "test_rows.rs"]
mod test_rows;
use self::observation_fixture::write_command_observation;
use self::test_rows::{changed_inputs, current_timing_row, digest, fmt_input};

#[test]
fn node_timing_reader_accepts_current_non_lane_rows_and_rejects_wrong_inputs() {
    let root = temp_root("live-loop-node-timing");
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context = context_digest();
    let input = fmt_input(candidate, &changed, &context);
    let current_row = current_timing_row(candidate, &input, "pass", "none");
    write_command_observation(&root, &current_row);
    crate::json_boundary::write_json(
        &root.join(NODE_TIMING_REL),
        &json!({
            "nodes": [
                current_row,
                {
                    "node_id": "build_check",
                    "candidate_digest": "sha256:stale",
                    "tier": "hot",
                    "cache_mode": "verified-local",
                    "input_digest": "sha256:wrong",
                    "canonical_full_command": "cargo build --offline --bin ultragoal --quiet",
                    "timing_status": "pass",
                    "cache_honesty": "pass",
                    "failure_class": "none",
                    "baseline_duration_ms": 1000,
                    "verified_local_duration_ms": 2
                }
            ]
        }),
    )
    .expect("timing artifact");

    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed_inputs(&changed, &context),
    );
    assert_eq!(timings.len(), 1);
    assert_eq!(
        timings["fmt_check"].affected_set_status,
        "clean_worktree_no_affected_files"
    );
    assert_eq!(timings["fmt_check"].timing_status, "pass");
    assert_eq!(timings["fmt_check"].failure_class, "none");
    assert_eq!(timings["fmt_check"].baseline_exit_code, Some(0));
    std::fs::remove_dir_all(root).expect("cleanup node timing");
}

#[test]
fn node_timing_reader_accepts_non_lane_verified_cache_hit_with_current_input_equivalence() {
    let root = temp_root("live-loop-prior-candidate-cache-timing");
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context = context_digest();
    let input = fmt_input(candidate, &changed, &context);
    let mut row = current_timing_row("sha256:prior", &input, "pass", "none");
    let output_digest = expected_output_digest();
    let result_digest = expected_result_digest(0, false, &output_digest);
    {
        let object = row.as_object_mut().expect("timing row object");
        object.insert("proof_kind".to_string(), json!("verified_cache_hit"));
        object.insert("cache_hit".to_string(), json!(true));
        object.insert("work_unit_count".to_string(), json!(0));
        object.insert(
            "equivalence_status".to_string(),
            json!("verified_same_candidate_cache_replay"),
        );
        object.insert(
            "invalidation_proof".to_string(),
            json!("cache_key_current_input_digest_command_contract_runtime_model_versions_and_environment_matched"),
        );
        object.insert("prior_result_digest".to_string(), json!(result_digest));
        object.insert("replayed_output_digest".to_string(), json!(output_digest));
        object.insert("cache_equivalence_status".to_string(), json!("pass"));
    }
    write_command_observation(&root, &row);
    crate::json_boundary::write_json(&root.join(NODE_TIMING_REL), &json!({ "nodes": [row] }))
        .expect("timing artifact");

    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed_inputs(&changed, &context),
    );
    assert_eq!(timings.len(), 1);
    assert_eq!(timings["fmt_check"].proof_kind, "verified_cache_hit");
    assert_eq!(
        timings["fmt_check"].equivalence_status,
        "verified_same_candidate_cache_replay"
    );
    std::fs::remove_dir_all(root).expect("cleanup prior-candidate timing");
}

#[test]
fn node_timing_reader_preserves_current_non_lane_failed_measurement_rows() {
    let root = temp_root("live-loop-failed-timing");
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context = context_digest();
    let input = fmt_input(candidate, &changed, &context);
    let failed_row = current_timing_row(candidate, &input, "fail", "canonical_full_command_failed");
    write_command_observation(&root, &failed_row);
    crate::json_boundary::write_json(
        &root.join(NODE_TIMING_REL),
        &json!({
            "nodes": [
                failed_row
            ]
        }),
    )
    .expect("timing artifact");

    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed_inputs(&changed, &context),
    );
    assert_eq!(timings.len(), 1);
    assert_eq!(timings["fmt_check"].timing_status, "fail");
    assert_eq!(
        timings["fmt_check"].failure_class,
        "canonical_full_command_failed"
    );
    assert_eq!(timings["fmt_check"].baseline_exit_code, Some(101));
    std::fs::remove_dir_all(root).expect("cleanup failed timing");
}

#[test]
fn node_timing_reader_rejects_legacy_key_only_speed_rows() {
    let root = temp_root("live-loop-legacy-timing");
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context = context_digest();
    let input = fmt_input(candidate, &changed, &context);
    crate::json_boundary::write_json(
        &root.join(NODE_TIMING_REL),
        &json!({
            "nodes": [{
                "node_id": "fmt_check",
                "candidate_digest": candidate,
                "tier": "hot",
                "cache_mode": "verified-local",
                "input_digest": input,
                "canonical_full_command": "cargo fmt --all --check",
                "timing_status": "pass",
                "cache_honesty": "pass",
                "failure_class": "none",
                "baseline_duration_ms": 1000,
                "verified_local_duration_ms": 1
            }]
        }),
    )
    .expect("legacy timing artifact");

    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed_inputs(&changed, &context),
    );
    assert!(timings.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup legacy timing");
}

#[test]
fn node_timing_reader_rejects_launched_rows_without_baseline_exit_code() {
    let root = temp_root("live-loop-missing-exit");
    let candidate = "sha256:current";
    let changed = crate::digest::bytes(b"");
    let context = context_digest();
    let input = fmt_input(candidate, &changed, &context);
    let mut row = current_timing_row(candidate, &input, "fail", "canonical_full_command_failed");
    write_command_observation(&root, &row);
    row.as_object_mut()
        .expect("timing row object")
        .remove("baseline_exit_code");
    crate::json_boundary::write_json(&root.join(NODE_TIMING_REL), &json!({ "nodes": [row] }))
        .expect("timing artifact");

    let timings = read_current(
        &root,
        candidate,
        "hot",
        "verified-local",
        &changed_inputs(&changed, &context),
    );
    assert!(timings.is_empty());
    std::fs::remove_dir_all(root).expect("cleanup missing-exit timing");
}

fn context_digest() -> String {
    crate::digest::bytes(
        format!(
            "validator={};law={};schema={};fixture={};tier=hot;cache=verified-local",
            crate::cli::live_loop::graph::validator_version(),
            crate::cli::live_loop::graph::law_version(),
            crate::cli::live_loop::graph::schema_version(),
            crate::cli::live_loop::graph::fixture_version()
        )
        .as_bytes(),
    )
}

fn expected_output_digest() -> String {
    let stdout_digest = digest("stdout");
    let stderr_digest = digest("stderr");
    crate::digest::bytes(format!("stdout={stdout_digest};stderr={stderr_digest}").as_bytes())
}

fn expected_result_digest(exit_code: i32, launch_error: bool, output_digest: &str) -> String {
    crate::digest::bytes(
        format!("exit={exit_code};launch={launch_error};output={output_digest}").as_bytes(),
    )
}
