use crate::audit::contract::Failure;
use crate::claim_semantics::{
    array_strings, canonical_digest, good_status, path_contains, str_field,
};
use crate::digest;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(crate) fn check_ready_join(cm: &Value, lr: &Value, ready: &Value, out: &mut Vec<Failure>) {
    let included = included_claim_ids(cm);
    if included.is_empty() {
        return;
    }
    readiness_flags(ready, out);
    let lanes = lr
        .get("lanes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|lane| str_field(lane, "id") == str_field(ready, "lane_id"))
        .collect::<Vec<_>>();
    if lanes.len() != 1 {
        out.push(Failure::new(
            "ready-receipt-provenance",
            "ready_receipt_not_lane_bound",
            str_field(ready, "lane_id"),
        ));
        return;
    }
    let lane = lanes[0];
    lane_field_join(ready, lane, out);
    claim_and_path_join(ready, lane, &included, out);
}

pub(crate) fn check_ready_integrity(lr: &Value, ready: &Value, out: &mut Vec<Failure>) {
    let expected_registry_digest = lane_registry_digest(lr);
    if str_field(ready, "lane_registry_digest") != expected_registry_digest {
        out.push(Failure::new(
            "ready-receipt-provenance",
            "ready_receipt_not_lane_bound",
            format!(
                "{}:lane_registry_digest expected {expected_registry_digest}",
                str_field(ready, "lane_id")
            ),
        ));
    }
    check_ready_digest(lr, ready, out);
}

fn lane_registry_digest(lr: &Value) -> String {
    let mut normalized = lr.clone();
    if let Some(lanes) = normalized.get_mut("lanes").and_then(Value::as_array_mut) {
        for lane in lanes {
            if let Some(receipt) = lane.get_mut("ready_receipt") {
                receipt["digest"] = json!(digest::ZERO);
            }
            if let Some(dependencies) = lane.get_mut("dependencies").and_then(Value::as_array_mut) {
                for dependency in dependencies {
                    dependency["evidence_digest"] = json!(digest::ZERO);
                    if let Some(receipt) = dependency.get_mut("upstream_ready_receipt") {
                        receipt["digest"] = json!(digest::ZERO);
                    }
                }
            }
        }
    }
    canonical_digest(&normalized).unwrap_or_default()
}

fn check_ready_digest(lr: &Value, ready: &Value, out: &mut Vec<Failure>) {
    let Some(lane) = lr
        .get("lanes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|lane| str_field(lane, "id") == str_field(ready, "lane_id"))
    else {
        return;
    };
    let expected = canonical_digest(ready).unwrap_or_default();
    if lane
        .pointer("/ready_receipt/digest")
        .and_then(Value::as_str)
        != Some(expected.as_str())
    {
        out.push(Failure::new(
            "ready-receipt-provenance",
            "ready_receipt_not_lane_bound",
            format!("{}:ready_receipt.digest", str_field(ready, "lane_id")),
        ));
    }
}

fn included_claim_ids(cm: &Value) -> BTreeSet<String> {
    cm.get("claims")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|claim| {
            str_field(claim, "claim_ceiling_effect") == "included"
                || good_status(&str_field(claim, "status"))
        })
        .map(|claim| str_field(claim, "id"))
        .collect()
}

fn readiness_flags(ready: &Value, out: &mut Vec<Failure>) {
    if ready.get("ready").and_then(Value::as_bool) != Some(true) {
        out.push(Failure::new(
            "ready-receipt-provenance",
            "ready_receipt_not_ready",
            str_field(ready, "lane_id"),
        ));
    }
    if ready
        .get("blocked_reasons")
        .and_then(Value::as_array)
        .is_some_and(|rows| !rows.is_empty())
    {
        out.push(Failure::new(
            "ready-receipt-provenance",
            "ready_receipt_has_blockers",
            str_field(ready, "lane_id"),
        ));
    }
    if ready.get("worktree_clean").and_then(Value::as_bool) != Some(true) {
        out.push(Failure::new(
            "worktree-teardown",
            "dirty_self_reported_clean",
            str_field(ready, "lane_id"),
        ));
    }
    if ready.get("teardown_ready").and_then(Value::as_bool) != Some(true) {
        out.push(Failure::new(
            "worktree-teardown",
            "stale_worktree_after_closeout",
            str_field(ready, "lane_id"),
        ));
    }
    if ready
        .get("commands")
        .and_then(Value::as_array)
        .is_none_or(|cmds| {
            cmds.is_empty()
                || cmds
                    .iter()
                    .any(|cmd| cmd.get("exit").and_then(Value::as_i64) != Some(0))
        })
    {
        out.push(Failure::new(
            "command-evidence",
            "command_receipt_missing_or_failed",
            str_field(ready, "lane_id"),
        ));
    }
}

fn lane_field_join(ready: &Value, lane: &Value, out: &mut Vec<Failure>) {
    for (ready_key, lane_key) in [
        ("workspace", "workspace"),
        ("branch", "branch"),
        ("base_commit", "base_commit"),
        ("commit", "current_commit"),
        ("target_branch", "target_branch"),
        ("target_head_at_launch", "target_head_at_launch"),
        ("target_head_at_validation", "target_head_at_validation"),
        ("merge_base", "merge_base_at_validation"),
        ("execplan", "execplan"),
    ] {
        if str_field(ready, ready_key) != str_field(lane, lane_key) {
            out.push(Failure::new(
                "ready-receipt-provenance",
                "ready_receipt_not_lane_bound",
                format!("{}:{ready_key}", str_field(ready, "lane_id")),
            ));
            return;
        }
    }
}

fn claim_and_path_join(
    ready: &Value,
    lane: &Value,
    included: &BTreeSet<String>,
    out: &mut Vec<Failure>,
) {
    let lane_claims = array_strings(lane, "claim_ids")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let ready_claims = array_strings(ready, "claim_ids")
        .into_iter()
        .collect::<BTreeSet<_>>();
    if ready_claims != *included || !included.is_subset(&lane_claims) {
        out.push(Failure::new(
            "ready-receipt-provenance",
            "ready_receipt_not_lane_bound",
            format!("{}:claim_ids", str_field(ready, "lane_id")),
        ));
    }
    if array_strings(ready, "changed_files").iter().any(|path| {
        !array_strings(lane, "owned_paths")
            .iter()
            .any(|owned| path_contains(owned, path))
    }) {
        out.push(Failure::new(
            "ready-receipt-provenance",
            "ready_receipt_not_lane_bound",
            format!("{}:changed_files", str_field(ready, "lane_id")),
        ));
    }
}
