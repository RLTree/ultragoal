use super::{ReplayFixture, digest, output_digest, result_digest, stdout_digest};
use serde_json::json;
use std::path::Path;

pub(super) fn command_observation_receipt(
    fixture: &ReplayFixture,
    command_text: &str,
    command_argv: &[String],
) -> serde_json::Value {
    json!({
        "schema": "harness-ultragoal.observe-receipt.v1",
        "status": "pass",
        "candidate_digest": fixture.candidate,
        "run_id": "run-test",
        "correlation_id": "corr-test",
        "trace_id": "trace-test",
        "command_identity": {
            "node_id": fixture.surface.id,
            "canonical_full_command": fixture.surface.canonical_full_command,
            "verified_local_command": command_text,
            "command_argv": command_argv
        },
        "process_result_authority": {
            "exit_status": 0,
            "status_success": true,
            "launch_error": false,
            "duration_ms": 100,
            "work_unit_count": 1,
            "stdout_digest": stdout_digest(),
            "stderr_digest": digest("stderr"),
            "output_digest": output_digest(),
            "result_digest": result_digest(),
            "redaction_status": "pass",
            "bounded_output_status": "digest_only_raw_output_not_retained",
            "claim_ceiling": "source_local_command_result_authority_only",
            "unsupported_claims": [
                "readiness",
                "release",
                "completion",
                "final_packet_correctness",
                "update_goal_eligibility"
            ]
        }
    })
}

pub(super) fn write_command_observation_receipt(root: &Path, row: &serde_json::Value) {
    let Some(receipt) = row
        .get("command_observation_receipt")
        .and_then(serde_json::Value::as_str)
    else {
        return;
    };
    let value = command_observation_receipt_from_row(row);
    let path = root.join(receipt);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("command observation receipt dir");
    }
    crate::json_boundary::write_json(&path, &value).expect("command observation receipt");
}

pub(super) fn command_observation_receipt_rel(node_id: &str) -> String {
    format!(
        "validation_artifacts/observability/live-loop/commands/{node_id}-command-observation.json"
    )
}

fn command_observation_receipt_from_row(row: &serde_json::Value) -> serde_json::Value {
    json!({
        "schema": "harness-ultragoal.observe-receipt.v1",
        "status": "pass",
        "candidate_digest": row["candidate_digest"],
        "run_id": "run-test",
        "correlation_id": "corr-test",
        "trace_id": "trace-test",
        "command_identity": {
            "node_id": row["node_id"],
            "canonical_full_command": row["canonical_full_command"],
            "verified_local_command": row["verified_local_command"],
            "command_argv": row["command_argv"]
        },
        "process_result_authority": {
            "exit_status": row["exit_status"],
            "status_success": row["exit_status"].as_i64() == Some(0),
            "launch_error": row["verified_local_launch_error"],
            "duration_ms": row["actual_work_duration_ms"],
            "work_unit_count": 1,
            "stdout_digest": row["verified_local_stdout_digest"],
            "stderr_digest": row["verified_local_stderr_digest"],
            "output_digest": output_digest(),
            "result_digest": result_digest(),
            "redaction_status": "pass",
            "bounded_output_status": "digest_only_raw_output_not_retained",
            "claim_ceiling": "source_local_command_result_authority_only",
            "unsupported_claims": [
                "readiness",
                "release",
                "completion",
                "final_packet_correctness",
                "update_goal_eligibility"
            ]
        }
    })
}
