use super::{NextActionCommand, next_action, parse, run};
use serde_json::json;
use std::fs;

#[test]
fn parser_accepts_summary_and_json_modes() {
    let command = parse(&["next".to_string(), "--json".to_string()])
        .expect("parse")
        .expect("next command");
    assert!(command.json);
    assert!(
        parse(&["current-state".to_string()])
            .expect("parse")
            .is_none()
    );
}

#[test]
fn next_action_combines_current_state_and_explain_authority() {
    let state = json!({
        "candidate_digest": "sha256:current",
        "active_stage": "custom_tooling_prerequisite",
        "git": {"dirty": false, "files": []},
        "first_blocker": {"id": "package digest", "why_failed": "missing trace proof"},
        "next_repair": "repair package digest telemetry",
        "narrow_rerun": "ultragoal observe command-roundtrip --command \"package digest\"",
        "claim_ceiling": "source_local_custom_tooling_prerequisite_only",
        "source_receipts": ["docs/generated/observability/command-inventory.json"]
    });
    let explain = json!({
        "claim_impact": "blocks_observability_speed_or_command_roundtrip",
        "forbidden_actions": ["do not call update_goal"],
        "blocked_claims": ["observability_speed_or_command_roundtrip"],
        "explanation": {
            "root_cause": "observability control board first incomplete row: commands/package digest",
            "implicated_paths": ["docs/generated/observability/command-inventory.json"],
            "smallest_repair": "repair trace proof and rerun command roundtrip",
            "narrow_rerun": "ultragoal observe command-roundtrip --command \"package digest\"",
            "broad_rerun": "source audit once after narrow proof passes"
        }
    });

    let action = next_action(&state, &explain);

    assert_eq!(action["status"], "fail");
    assert_eq!(action["candidate_digest"], "sha256:current");
    assert_eq!(action["active_stage"], "custom_tooling_prerequisite");
    assert_eq!(
        action["blocked_claim"],
        "blocks_observability_speed_or_command_roundtrip"
    );
    assert_eq!(
        action["smallest_repair"],
        "repair trace proof and rerun command roundtrip"
    );
    assert_eq!(
        action["authority_sources"]["current_state"],
        "ultragoal current-state --json"
    );
}

#[test]
fn next_run_rejects_non_package_root_before_planning() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("next-action-bad-root");
    fs::create_dir_all(&root).expect("root");
    let error = run(&root, &NextActionCommand { json: true })
        .expect_err("next rejects roots without package boundary");
    assert!(error.contains("plugin-manifest-draft.json"), "{error}");
    fs::remove_dir_all(root).expect("cleanup next bad root");
}

#[test]
fn next_run_prints_json_from_real_current_state_and_explain() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("next-action-run");
    fs::create_dir_all(root.join("docs/generated/observability")).expect("inventory dir");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    crate::json_boundary::write_json(
        &root.join("docs/generated/observability/command-inventory.json"),
        &json!({
            "observability_control_board": {
                "status": "blocked",
                "first_incomplete": {
                    "family": "commands",
                    "id": "package digest",
                    "observability_status": "unobservable",
                    "next_unobservable_surface": "same-candidate trace proof"
                }
            },
            "command_observability_inventory": {
                "package digest": {
                    "observability_status": "unobservable",
                    "missing_surfaces": ["same-candidate trace proof"],
                    "current_owner_surface": "command:package digest",
                    "claim_impact": "blocks_observability_speed_or_command_roundtrip"
                }
            }
        }),
    )
    .expect("inventory");

    let code = run(&root, &NextActionCommand { json: true }).expect("next run");

    assert_eq!(code, 1);
    fs::remove_dir_all(root).expect("cleanup next run");
}
