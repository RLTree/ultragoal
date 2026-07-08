use super::fixtures::{command, live_loop_timing_receipt_arg, live_loop_timing_receipt_path};
use crate::cli::live_loop::nodes::timing::VALIDATION_CACHE_REL;
use crate::self_tests::boundaries::workspace_fixtures::temp_root;
use serde_json::json;

#[test]
fn live_loop_measure_writes_current_node_timing_from_real_command_surface() {
    let root = temp_root("live-loop-measure-command");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let receipt = live_loop_timing_receipt_path(&root);
    let command = command(Some("changed_files"), live_loop_timing_receipt_arg());

    let first_code = crate::cli::live_loop::run(&root, &command).expect("first measure command");
    assert!(matches!(first_code, 0 | 1));
    let first_timing = crate::json_boundary::read_json(&receipt).expect("first timing receipt");
    let first_rows = first_timing["nodes"].as_array().expect("first nodes");
    assert_eq!(first_rows.len(), 1);
    assert_eq!(first_rows[0]["node_id"], "changed_files");
    let validation_cache = crate::json_boundary::read_json(&root.join(VALIDATION_CACHE_REL))
        .expect("validation cache");
    assert_eq!(
        validation_cache["schema"],
        "harness-ultragoal.live-loop-validation-cache.v1"
    );
    assert_eq!(validation_cache["candidate_digest"], candidate);
    assert!(
        validation_cache["records"]
            .as_array()
            .expect("cache records")
            .iter()
            .any(|row| row["node_id"] == "changed_files")
    );

    crate::json_boundary::write_json(
        &receipt,
        &json!({
            "nodes": [{
                "node_id": "stale_node",
                "candidate_digest": "sha256:stale",
                "tier": "hot",
                "cache_mode": "verified-local"
            }, {
                "node_id": "existing_current_node",
                "candidate_digest": candidate,
                "tier": "hot",
                "cache_mode": "verified-local"
            }]
        }),
    )
    .expect("stale timing");

    let code = crate::cli::live_loop::run(&root, &command).expect("measure command");
    assert!(matches!(code, 0 | 1));
    let timing = crate::json_boundary::read_json(&receipt).expect("timing receipt");
    assert_eq!(
        timing["schema"],
        "harness-ultragoal.live-loop-node-timing.v1"
    );
    assert_eq!(timing["candidate_digest"], candidate);
    let rows = timing["nodes"].as_array().expect("nodes");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["node_id"], "changed_files");
    assert_eq!(rows[1]["node_id"], "existing_current_node");
    assert_eq!(
        rows[0]["canonical_full_command"],
        "git status --short --untracked-files=all"
    );
    assert_eq!(rows[0]["baseline_exit_code"], 0);
    assert_command_receipt_state(&rows[0]);

    std::fs::remove_dir_all(root).expect("cleanup measure command");
}

fn assert_command_receipt_state(row: &serde_json::Value) {
    match row["proof_kind"].as_str().expect("proof kind") {
        "executed" => assert_executed_receipt_state(row),
        "verified_cache_hit" => assert_cache_replay_receipt_state(row),
        proof_kind => panic!("{proof_kind}"),
    }
    assert_result_digests(row);
}

fn assert_executed_receipt_state(row: &serde_json::Value) {
    let failure_class = row["failure_class"].as_str().expect("failure class");
    assert!(
        [
            "live_loop_telemetry_reconciliation_missing",
            "live_loop_speedup_target_missed"
        ]
        .contains(&failure_class),
        "{failure_class}"
    );
    assert_eq!(row["validation_status"], "pass");
    assert!(
        matches!(
            row["validation_cache_status"].as_str().expect("cache"),
            "reusable" | "unknown"
        ),
        "{}",
        row["validation_cache_status"]
    );
    assert_eq!(row["cache_hit"], false);
    assert_eq!(row["work_unit_count"], 1);
    let telemetry_status = row["telemetry_reconciliation_status"]
        .as_str()
        .expect("telemetry status");
    assert!(
        matches!(
            telemetry_status,
            "query_or_explain_reconciliation_failed" | "pass" | "command_observation_failed"
        ),
        "{telemetry_status}"
    );
    assert_ne!(telemetry_status, "missing_command_telemetry");
    if telemetry_status != "command_observation_failed" {
        assert_eq!(
            row["telemetry_reconciliation"]["command_observation"]["event"]["operation"],
            "loop.measure.changed_files"
        );
        assert!(
            row["telemetry_reconciliation"]["command_observation_receipt"]
                .as_str()
                .expect("command observation receipt")
                .contains("changed_files-command-observation.json")
        );
    } else {
        assert_eq!(row["observability_status"], "unavailable");
        assert_eq!(row["speed_claim_status"], "withheld");
    }
}

fn assert_cache_replay_receipt_state(row: &serde_json::Value) {
    assert_eq!(row["failure_class"], "none");
    assert_eq!(row["validation_status"], "pass");
    assert_eq!(row["validation_cache_status"], "reusable");
    assert_eq!(row["observability_status"], "pass");
    assert_eq!(row["speed_claim_status"], "supported");
    assert_eq!(row["cache_hit"], true);
    assert_eq!(row["work_unit_count"], 0);
    assert_eq!(
        row["equivalence_status"],
        "verified_same_candidate_cache_replay"
    );
    assert_eq!(row["cache_equivalence_status"], "pass");
    assert_eq!(row["prior_result_digest"], row["result_digest"]);
    assert_eq!(row["replayed_output_digest"], row["output_digest"]);
    assert_eq!(row["telemetry_reconciliation"]["status"], "pass");
    assert_eq!(
        row["telemetry_reconciliation"]["reconciliation_mode"],
        "verified_same_candidate_telemetry_reuse"
    );
}

fn assert_result_digests(row: &serde_json::Value) {
    assert!(
        row["result_digest"]
            .as_str()
            .expect("result digest")
            .starts_with("sha256:")
    );
    assert_eq!(row["result_digest"], row["verified_local_result_digest"]);
    assert_eq!(row["output_digest"], row["verified_local_output_digest"]);
    assert!(
        row["verified_local_result_digest"]
            .as_str()
            .expect("result digest")
            .starts_with("sha256:")
    );
    let affected_set_status = row["affected_set_status"]
        .as_str()
        .expect("affected set status");
    assert!(
        matches!(
            affected_set_status,
            "clean_worktree_no_affected_files" | "changed_files_digest_bound"
        ),
        "{affected_set_status}"
    );
}
