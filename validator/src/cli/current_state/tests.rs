use super::{
    CurrentStateCommand, digest_field, first_blocker, git_status, parse, receipt_state, run,
};
use serde_json::json;

#[test]
fn parser_defaults_to_bounded_current_state_receipt() {
    let command = parse(&["current-state".to_string(), "--json".to_string()])
        .expect("parse")
        .expect("command");
    assert!(command.json);
    assert_eq!(
        command.receipt.to_string_lossy(),
        "validation_artifacts/current-state.json"
    );
    assert!(parse(&["package".to_string()]).expect("parse").is_none());
}

#[test]
fn digest_fields_cover_known_receipt_shapes() {
    assert_eq!(
        digest_field(&json!({"target_revision": {"value": "sha256:a"}})),
        Some("sha256:a")
    );
    assert_eq!(
        digest_field(&json!({"target_digest": "sha256:b"})),
        Some("sha256:b")
    );
    assert_eq!(
        digest_field(&json!({"candidate_digest": "sha256:c"})),
        Some("sha256:c")
    );
    assert_eq!(digest_field(&json!({})), None);
}

#[test]
fn receipt_state_reports_current_status_detail_and_missing_digest() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("current-state-receipt-state");
    std::fs::create_dir_all(root.join("validation_artifacts/coverage")).expect("receipt dir");
    crate::json_boundary::write_json(
        &root.join("validation_artifacts/coverage/coverage-receipt.json"),
        &json!({
            "status": "pass",
            "target_digest": "sha256:current",
            "failures": [{"detail": "covered"}]
        }),
    )
    .expect("receipt");
    let current = receipt_state(
        &root,
        "validation_artifacts/coverage/coverage-receipt.json",
        "sha256:current",
    );
    assert_eq!(current["status"], "pass");
    assert_eq!(current["current"], true);
    assert_eq!(current["first_detail"], "covered");

    crate::json_boundary::write_json(
        &root.join("validation_artifacts/coverage/no-digest.json"),
        &json!({"details": ["missing digest detail"]}),
    )
    .expect("missing digest receipt");
    let missing_digest = receipt_state(
        &root,
        "validation_artifacts/coverage/no-digest.json",
        "sha256:current",
    );
    assert_eq!(missing_digest["status"], "unknown");
    assert_eq!(missing_digest["target_digest"], "missing");
    assert_eq!(missing_digest["first_detail"], "missing digest detail");
    std::fs::remove_dir_all(root).expect("cleanup receipt state");
}

#[test]
fn git_status_reports_command_failure_as_dirty() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "current-state-missing-git-root",
    );
    let status = git_status(&root);
    assert_eq!(status["dirty"], true);
    assert!(status["error"].as_str().unwrap().contains("No such file"));
}

#[test]
fn first_blocker_prefers_observability_board_then_stale_receipt() {
    let board = json!({
        "status": "blocked",
        "first_incomplete": {"family": "commands", "id": "package digest"}
    });
    let pass = json!({"status": "pass", "current": true, "path": "ok"});
    assert_eq!(
        first_blocker(&board, &pass, &pass, &pass)["id"],
        "package digest"
    );
    let observable = json!({"status": "observable"});
    let stale = json!({"status": "fail", "current": false, "path": "coverage"});
    assert_eq!(
        first_blocker(&observable, &stale, &pass, &pass)["id"],
        "coverage"
    );
    assert_eq!(
        first_blocker(&observable, &pass, &pass, &pass)["id"],
        "none"
    );
}

#[test]
fn current_state_run_writes_json_and_summary_modes() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("current-state-run");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    for json_mode in [false, true] {
        let command = CurrentStateCommand {
            json: json_mode,
            receipt: format!("validation_artifacts/current-state-{json_mode}.json").into(),
        };
        let code = run(&root, &command).expect("current state run");
        assert_eq!(code, 1);
        assert!(root.join(&command.receipt).is_file());
    }
}

#[test]
fn current_state_run_fails_closed_without_candidate_digest() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "current-state-missing-package-boundary",
    );
    let command = CurrentStateCommand {
        json: true,
        receipt: "validation_artifacts/current-state-missing-boundary.json".into(),
    };

    let err = run(&root, &command).expect_err("missing candidate digest must fail closed");

    assert!(
        err.contains("plugin-manifest-draft.json") || err.contains("No such file"),
        "candidate digest failure should name the missing package boundary: {err}"
    );
    assert!(
        !root.join(&command.receipt).exists(),
        "current-state must not mint a receipt when candidate truth is unavailable"
    );
}
