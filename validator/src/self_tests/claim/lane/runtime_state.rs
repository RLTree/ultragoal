use crate::audit::contract::Failure;
use serde_json::{Value, json};

fn base_lane() -> Value {
    json!({
        "id": "LANE-1",
        "workspace": "worktrees/lane-1",
        "branch": "lane-1",
        "current_commit": "commit-a",
        "target_head_at_validation": "target-a",
        "target_head_at_launch": "target-a",
        "merge_base_at_validation": "target-a",
        "target_freshness": {"live_target_receipt": null},
        "workspace_status": {},
        "teardown": {"required": false, "status": "complete", "cleanup_receipts": []}
    })
}

fn errors(out: &[Failure]) -> Vec<String> {
    out.iter().map(|failure| failure.error.clone()).collect()
}

#[test]
fn lane_target_and_worktree_status_fail_closed() {
    let mut out = Vec::new();
    crate::claim_semantics::lane::runtime::state::target_freshness(&base_lane(), &mut out);
    assert_eq!(errors(&out), vec!["live_target_receipt_missing"]);
    let mut lane = base_lane();
    lane["target_freshness"]["live_target_receipt"] = json!({
        "command": {"exit": 1},
        "observed_workspace": "other",
        "observed_target_head": "target-b",
        "observed_merge_base": "target-a"
    });
    out.clear();
    crate::claim_semantics::lane::runtime::state::target_freshness(&lane, &mut out);
    let got = errors(&out);
    assert!(got.contains(&"live_target_receipt_missing".to_string()));
    assert!(got.contains(&"wrong_lane_worktree_root".to_string()));
    assert!(got.contains(&"target_head_changed_after_validation".to_string()));
    lane["target_freshness"]["live_target_receipt"] = json!({
        "command": {"exit": 0},
        "observed_workspace": "worktrees/lane-1",
        "observed_target_head": "target-a",
        "observed_merge_base": "old-base"
    });
    out.clear();
    crate::claim_semantics::lane::runtime::state::target_freshness(&lane, &mut out);
    assert_eq!(errors(&out), vec!["merge_base_behind_target"]);

    let mut status_lane = base_lane();
    out.clear();
    crate::claim_semantics::lane::runtime::state::worktree_status(&status_lane, &mut out);
    assert_eq!(errors(&out), vec!["worktree_status_command_failed"]);
    status_lane["workspace_status"] = json!({
        "status_command_receipt": {"exit": 2},
        "observed_workspace": "other",
        "observed_branch": "wrong",
        "observed_head": "wrong",
        "clean": true,
        "observed_clean": false,
        "dirty_paths": ["src/lib.rs"]
    });
    out.clear();
    crate::claim_semantics::lane::runtime::state::worktree_status(&status_lane, &mut out);
    let got = errors(&out);
    assert!(got.contains(&"worktree_status_command_failed".to_string()));
    assert!(got.contains(&"wrong_lane_worktree_root".to_string()));
    assert!(got.contains(&"dirty_self_reported_clean".to_string()));
}

#[test]
fn lane_teardown_validates_cleanup_receipts_and_digests() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("lane-runtime");
    std::fs::create_dir_all(root.join("receipts")).expect("receipts");
    let receipt = root.join("receipts/cleanup.json");
    let audit = root.join("receipts/post-clean.json");
    std::fs::write(&receipt, "{}").expect("receipt");
    std::fs::write(&audit, "{}").expect("audit");
    let receipt_digest = crate::digest::file(&receipt).expect("receipt digest");
    let audit_digest = crate::digest::file(&audit).expect("audit digest");

    let mut lane = base_lane();
    lane["teardown"] = json!({
        "required": true,
        "status": "not_ready",
        "cleanup_receipts": [{
            "receipt": {"path": "receipts/cleanup.json", "digest": receipt_digest},
            "post_clean_audit": {"path": "receipts/post-clean.json", "digest": audit_digest}
        }]
    });
    let mut out = Vec::new();
    crate::claim_semantics::lane::runtime::state::teardown(&lane, &root, &mut out);
    assert_eq!(errors(&out), vec!["stale_worktree_after_closeout"]);

    lane["teardown"]["status"] = json!("complete");
    lane["teardown"]["cleanup_receipts"][0]["receipt"]["digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('0'));
    out.clear();
    crate::claim_semantics::lane::runtime::state::teardown(&lane, &root, &mut out);
    assert_eq!(errors(&out), vec!["terminal_cleanup_without_receipt"]);

    lane["teardown"]["cleanup_receipts"] = json!([{"post_clean_audit": {
        "path": "receipts/post-clean.json",
        "digest": audit_digest
    }}]);
    out.clear();
    crate::claim_semantics::lane::runtime::state::teardown(&lane, &root, &mut out);
    assert_eq!(errors(&out), vec!["terminal_cleanup_without_receipt"]);

    lane["teardown"]["cleanup_receipts"] = json!([{
        "receipt": {"path": "../escape.json", "digest": receipt_digest},
        "post_clean_audit": {"path": "receipts/post-clean.json", "digest": audit_digest}
    }]);
    out.clear();
    crate::claim_semantics::lane::runtime::state::teardown(&lane, &root, &mut out);
    assert_eq!(errors(&out), vec!["terminal_cleanup_without_receipt"]);

    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn lane_runtime_accepts_current_target_status_and_cleanup_receipts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("lane-runtime-current");
    std::fs::create_dir_all(root.join("receipts")).expect("receipts");
    let receipt = root.join("receipts/cleanup.json");
    let audit = root.join("receipts/post-clean.json");
    std::fs::write(&receipt, "{}").expect("receipt");
    std::fs::write(&audit, "{}").expect("audit");
    let receipt_digest = crate::digest::file(&receipt).expect("receipt digest");
    let audit_digest = crate::digest::file(&audit).expect("audit digest");
    let mut lane = base_lane();
    lane["target_freshness"]["live_target_receipt"] = json!({
        "command": {"exit": 0},
        "observed_workspace": "worktrees/lane-1",
        "observed_target_head": "target-a",
        "observed_merge_base": "target-a"
    });
    lane["workspace_status"] = json!({
        "status_command_receipt": {"exit": 0},
        "observed_workspace": "worktrees/lane-1",
        "observed_branch": "lane-1",
        "observed_head": "commit-a",
        "clean": false,
        "observed_clean": false,
        "dirty_paths": ["expected-local-change"]
    });
    lane["teardown"] = json!({
        "required": true,
        "status": "complete",
        "cleanup_receipts": [{
            "receipt": {"path": "receipts/cleanup.json", "digest": receipt_digest},
            "post_clean_audit": {"path": "receipts/post-clean.json", "digest": audit_digest}
        }]
    });
    let mut out = Vec::new();
    crate::claim_semantics::lane::runtime::state::target_freshness(&lane, &mut out);
    crate::claim_semantics::lane::runtime::state::worktree_status(&lane, &mut out);
    crate::claim_semantics::lane::runtime::state::teardown(&lane, &root, &mut out);
    assert!(out.is_empty(), "{out:?}");
    std::fs::remove_dir_all(root).expect("cleanup current lane runtime");
}
