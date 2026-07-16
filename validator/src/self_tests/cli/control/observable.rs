use crate::cli::control::plane::operation::ControlOperation;
use crate::cli::control::plane::{ControlCommand, run};
use serde_json::json;
use std::path::Path;

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn self_update_goal_failures_emit_observable_repair_contract() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("cli-self-update-observable");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"0.0.0-test","resources":[]}),
    );
    let path = Path::new("validation_artifacts/cli/self-law-receipt.json");
    let command = ControlCommand {
        operation: ControlOperation::SelfUpdateGoalEligibility,
        receipt: Some(path.to_path_buf()),
        surface_root: None,
        agent_authority_roots: None,
    };

    assert_eq!(run(&root, &command).expect("run writes receipt"), 1);

    let receipt_path = root.join(path);
    let value = crate::json_boundary::read_json(&receipt_path).expect("read self-law receipt");
    assert_eq!(value["operation"], "self_update_goal_eligibility");
    assert_eq!(
        value["check_id"],
        "self-update-goal-eligibility-observability-binding"
    );
    assert_eq!(value["claim_id"], "update_goal_eligibility");
    assert_eq!(
        value["observability"]["surface"],
        "source_local_update_goal_control_plane"
    );
    assert_eq!(
        value["receipt_observability_binding"]["command_receipt_path"],
        "validation_artifacts/cli/self-law-receipt.json"
    );
    for key in ["run_id", "correlation_id", "trace_id", "span_id"] {
        assert!(
            value.get(key).and_then(serde_json::Value::as_str).is_some(),
            "{key} missing from {value}"
        );
    }
    assert_eq!(value["parent_span_id"], "");
    assert!(value["duration_ms"].as_u64().expect("duration") > 0);
    assert!(
        value["why_failed"]
            .as_str()
            .expect("why")
            .contains("evidence_graph"),
        "{value}"
    );
    assert!(
        value["next_repair"]
            .as_str()
            .expect("next repair")
            .contains("self update-goal eligibility"),
        "{value}"
    );
    let run_id = value["run_id"].as_str().expect("run id");
    let lines = crate::cli::control::plane::registry::stdout::lines(&receipt_path, &value);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("ultragoal-control fail"));
    assert!(lines[0].contains("operation=self_update_goal_eligibility"));
    assert!(lines[1].contains("failed_check=self-update-goal-eligibility-observability-binding"));
    assert!(lines[1].contains(&format!(
        "query_logs='ultragoal observe logs query --run-id {run_id} --limit 100'"
    )));
    std::fs::remove_dir_all(root).expect("cleanup cli self update observable");
}
