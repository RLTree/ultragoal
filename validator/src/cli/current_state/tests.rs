use super::{
    CurrentStateCommand, first_blocker, git_status, observability_control_board, parse,
    receipt_state, run, snapshot_for_candidate,
};
use serde_json::json;

fn metadata_snapshot(root: &std::path::Path) -> Vec<(std::path::PathBuf, u64, bool)> {
    let mut rows = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .map(Result::unwrap)
        .map(|entry| {
            let metadata = std::fs::symlink_metadata(entry.path()).expect("metadata");
            (
                entry.path().strip_prefix(root).unwrap().to_path_buf(),
                metadata.len(),
                metadata.file_type().is_symlink(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

#[test]
fn parser_defaults_to_read_only_and_accepts_explicit_receipt() {
    let command = parse(&["current-state".to_string(), "--json".to_string()])
        .expect("parse")
        .expect("command");
    assert!(command.json);
    assert!(command.receipt.is_none());
    let explicit = parse(&[
        "current-state".to_string(),
        "--receipt".to_string(),
        "validation_artifacts/current-state.json".to_string(),
    ])
    .expect("parse")
    .expect("command");
    assert_eq!(
        explicit.receipt.as_deref(),
        Some(std::path::Path::new(
            "validation_artifacts/current-state.json"
        ))
    );
    assert!(parse(&["package".to_string()]).expect("parse").is_none());
}

#[test]
fn digest_fields_cover_known_receipt_shapes() {
    assert_eq!(
        super::receipt_status::digest_field(&json!({"target_revision": {"value": "sha256:a"}})),
        Some("sha256:a")
    );
    assert_eq!(
        super::receipt_status::digest_field(&json!({"target_digest": "sha256:b"})),
        Some("sha256:b")
    );
    assert_eq!(
        super::receipt_status::digest_field(&json!({"candidate_digest": "sha256:c"})),
        Some("sha256:c")
    );
    assert_eq!(super::receipt_status::digest_field(&json!({})), None);
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
        "status": "unavailable",
        "why_failed": "HCT-OBSERVE successor catalog unavailable/not adopted"
    });
    let pass = json!({"status": "pass", "current": true, "path": "ok"});
    assert_eq!(
        first_blocker(&board, &pass, &pass, &pass)["id"],
        "HCT-OBSERVE"
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
fn current_state_never_reads_static_command_inventory_or_claims_observable() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "current-state-static-inventory-bait",
    );
    let directory = root.join("docs/generated/observability");
    std::fs::create_dir_all(&directory).expect("directory");
    let path = directory.join("command-inventory.json");

    std::fs::write(&path, "SECRET_CANARY").expect("bait");
    let first = observability_control_board(&root);
    let first_state = snapshot_for_candidate(&root, "sha256:current".to_string());
    std::fs::write(&path, [0xff, 0xfe]).expect("invalid bytes");
    let second = observability_control_board(&root);
    std::fs::remove_file(&path).expect("remove bait");
    let missing = observability_control_board(&root);

    assert_eq!(first, second);
    assert_eq!(second, missing);
    assert_eq!(first["status"], "unavailable");
    assert_eq!(first_state["status"], "fail");
    assert_eq!(first_state["first_blocker"]["id"], "HCT-OBSERVE");
    assert!(!first_state.to_string().contains("SECRET_CANARY"));
    assert!(!root.join("validation_artifacts").exists());
    std::fs::remove_dir_all(root).expect("cleanup");
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
            receipt: Some(format!("validation_artifacts/current-state-{json_mode}.json").into()),
        };
        let code = run(&root, &command).expect("current state run");
        assert_eq!(code, 1);
        assert!(root.join(command.receipt.as_ref().unwrap()).is_file());
    }
}

#[test]
fn current_state_default_public_path_is_read_only() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("current-state-read-only");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let before = metadata_snapshot(&root);
    let command = CurrentStateCommand {
        json: true,
        receipt: None,
    };
    assert_eq!(run(&root, &command).expect("read-only current state"), 1);
    assert_eq!(before, metadata_snapshot(&root));
    assert!(!root.join("validation_artifacts").exists());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn current_state_run_fails_closed_without_candidate_digest() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "current-state-missing-package-boundary",
    );
    let command = CurrentStateCommand {
        json: true,
        receipt: Some("validation_artifacts/current-state-missing-boundary.json".into()),
    };

    let err = run(&root, &command).expect_err("missing candidate digest must fail closed");

    assert!(
        err.contains("plugin-manifest-draft.json") || err.contains("No such file"),
        "candidate digest failure should name the missing package boundary: {err}"
    );
    assert!(
        !root.join(command.receipt.as_ref().unwrap()).exists(),
        "current-state must not mint a receipt when candidate truth is unavailable"
    );
}

#[test]
fn current_state_run_rejects_external_claim_receipts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "current-state-external-receipt",
    );
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let command = CurrentStateCommand {
        json: false,
        receipt: Some(std::env::temp_dir().join("current-state-claim.json")),
    };

    let err = run(&root, &command).expect_err("external claim receipt must fail closed");

    assert!(
        err.contains("must be a root-relative claim artifact path"),
        "absolute current-state receipts must not support claims: {err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup external receipt");
}

#[test]
fn current_state_run_fails_closed_when_receipt_cannot_be_written() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("current-state-write-failure");
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    std::fs::write(root.join("validation_artifacts"), b"file").expect("block receipt directory");
    let command = CurrentStateCommand {
        json: false,
        receipt: Some("validation_artifacts/current-state.json".into()),
    };

    let err = run(&root, &command).expect_err("receipt write must fail closed");

    assert!(
        err.contains("validation_artifacts"),
        "receipt write failure should name the blocked artifact path: {err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup write failure");
}
