use crate::audit::contract::Failure;
use crate::claim_semantics::{bool_field, str_field};
use crate::digest;
use serde_json::Value;
use std::path::Path;

pub(crate) fn target_freshness(lane: &Value, out: &mut Vec<Failure>) {
    let live = &lane["target_freshness"]["live_target_receipt"];
    if live.is_null() {
        out.push(Failure::new(
            "lane-target-freshness",
            "live_target_receipt_missing",
            str_field(lane, "id"),
        ));
        return;
    }
    if live.pointer("/command/exit").and_then(Value::as_i64) != Some(0) {
        out.push(Failure::new(
            "lane-target-freshness",
            "live_target_receipt_missing",
            str_field(lane, "id"),
        ));
    }
    if str_field(live, "observed_workspace") != str_field(lane, "workspace") {
        out.push(Failure::new(
            "worktree-teardown",
            "wrong_lane_worktree_root",
            str_field(lane, "id"),
        ));
    }
    target_commit_freshness(live, lane, out);
}

pub(crate) fn worktree_status(lane: &Value, out: &mut Vec<Failure>) {
    let status = &lane["workspace_status"];
    if status.get("status_command_receipt").is_none() {
        out.push(Failure::new(
            "worktree-teardown",
            "worktree_status_command_failed",
            str_field(lane, "id"),
        ));
        return;
    }
    if status
        .pointer("/status_command_receipt/exit")
        .and_then(Value::as_i64)
        != Some(0)
    {
        out.push(Failure::new(
            "worktree-teardown",
            "worktree_status_command_failed",
            str_field(lane, "id"),
        ));
    }
    if str_field(status, "observed_workspace") != str_field(lane, "workspace")
        || str_field(status, "observed_branch") != str_field(lane, "branch")
        || str_field(status, "observed_head") != str_field(lane, "current_commit")
    {
        out.push(Failure::new(
            "worktree-teardown",
            "wrong_lane_worktree_root",
            str_field(lane, "id"),
        ));
    }
    if bool_field(status, "clean") != bool_field(status, "observed_clean")
        || (bool_field(status, "clean")
            && status
                .get("dirty_paths")
                .and_then(Value::as_array)
                .is_some_and(|p| !p.is_empty()))
    {
        out.push(Failure::new(
            "worktree-teardown",
            "dirty_self_reported_clean",
            str_field(lane, "id"),
        ));
    }
}

pub(crate) fn teardown(lane: &Value, root: &Path, out: &mut Vec<Failure>) {
    if lane.pointer("/teardown/required").and_then(Value::as_bool) == Some(true)
        && lane.pointer("/teardown/status").and_then(Value::as_str) == Some("not_ready")
    {
        out.push(Failure::new(
            "worktree-teardown",
            "stale_worktree_after_closeout",
            str_field(lane, "id"),
        ));
    }
    for clean in lane
        .pointer("/teardown/cleanup_receipts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if cleanup_ref_invalid(root, clean.pointer("/receipt"))
            || cleanup_ref_invalid(root, clean.pointer("/post_clean_audit"))
        {
            out.push(Failure::new(
                "worktree-teardown",
                "terminal_cleanup_without_receipt",
                str_field(lane, "id"),
            ));
        }
    }
}

fn cleanup_ref_invalid(root: &Path, artifact: Option<&Value>) -> bool {
    let Some(artifact) = artifact else {
        return true;
    };
    let rel = artifact.get("path").and_then(Value::as_str).unwrap_or("");
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        return true;
    }
    let digest_value = artifact.get("digest").and_then(Value::as_str).unwrap_or("");
    if digest_value == digest::ZERO || !digest_value.starts_with("sha256:") {
        return true;
    }
    digest::file(&root.join(rel)).map_or(true, |actual| actual != digest_value)
}

fn target_commit_freshness(live: &Value, lane: &Value, out: &mut Vec<Failure>) {
    if str_field(live, "observed_target_head") != str_field(lane, "target_head_at_validation")
        || str_field(lane, "target_head_at_validation") != str_field(lane, "target_head_at_launch")
    {
        out.push(Failure::new(
            "lane-target-freshness",
            "target_head_changed_after_validation",
            str_field(lane, "id"),
        ));
    } else if str_field(live, "observed_merge_base") != str_field(lane, "merge_base_at_validation")
        || str_field(lane, "merge_base_at_validation")
            != str_field(lane, "target_head_at_validation")
    {
        out.push(Failure::new(
            "lane-target-freshness",
            "merge_base_behind_target",
            str_field(lane, "id"),
        ));
    }
}
