use serde_json::json;
use std::path::Path;

use super::test_rows::FMT_COMMAND_OBSERVATION_REL;

pub(super) fn write_command_observation(root: &Path, row: &serde_json::Value) {
    let node_id = row["node_id"].as_str().expect("node id");
    let candidate = row["candidate_digest"].as_str().expect("candidate");
    let argv = row["command_argv"].as_array().expect("argv");
    let command = argv[0].as_str().expect("command");
    let subcommand = argv
        .iter()
        .skip(1)
        .map(|item| item.as_str().expect("argv item"))
        .collect::<Vec<_>>()
        .join(" ");
    let output_digest = row["output_digest"].as_str().expect("output digest");
    let result_digest = row["result_digest"].as_str().expect("result digest");
    let rel = FMT_COMMAND_OBSERVATION_REL;
    let executed_test_count = row
        .get("verified_local_executed_test_count")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let receipt = json!({
        "schema": "harness-ultragoal.observability-receipt.v1",
        "status": "pass",
        "candidate_digest": candidate,
        "receipt_path": rel,
        "operation": format!("loop.measure.{node_id}"),
        "event": {
            "status": "pass",
            "candidate_digest": candidate,
            "artifact_path": super::super::NODE_TIMING_REL,
            "operation": format!("loop.measure.{node_id}"),
            "command": command,
            "subcommand": subcommand
        },
        "command_result_authority": {
            "authority": "live_loop_command_observation_actual_work",
            "exit_status": row["exit_status"],
            "launch_error": row["verified_local_launch_error"],
            "stdout_digest": row["verified_local_stdout_digest"],
            "stderr_digest": row["verified_local_stderr_digest"],
            "output_digest": output_digest,
            "result_digest": result_digest,
            "executed_test_count": executed_test_count
        }
    });
    crate::json_boundary::write_json(&root.join(rel), &receipt)
        .expect("command observation receipt");
}
