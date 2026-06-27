use crate::audit::contract::Failure;
use crate::claim_semantics::{array_strings, canonical_digest, str_field};
use crate::digest;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) fn check(
    lane: &Value,
    dep: &Value,
    lane_index: &BTreeMap<String, &Value>,
    ready_index: &BTreeMap<String, &Value>,
    root_phases: &Value,
    bundle: &Value,
    root: &Path,
    out: &mut Vec<Failure>,
) {
    let Some(upstream) = lane_index.get(&str_field(dep, "upstream_lane_id")) else {
        push_failure(dep, out);
        return;
    };
    if basic_dependency_bad(dep, upstream, root_phases) {
        push_failure(dep, out);
        return;
    }
    let receipt = ready_index.get(&str_field(upstream, "id"));
    if receipt.is_none() {
        push_ready_failure(dep, out);
        return;
    }
    if receipt.is_some_and(|ready| ready_authority_bad(dep, upstream, ready)) {
        push_ready_failure(dep, out);
        return;
    }
    if receipt.is_some_and(|ready| {
        !crate::claim_semantics::ready::receipt::generated_ready_artifact_ok(bundle, ready, root)
    }) {
        out.push(
            crate::claim_semantics::ready::receipt::ready_output_failure(&str_field(
                dep,
                "upstream_lane_id",
            )),
        );
        return;
    }
    if receipt.is_some_and(|ready| dependency_order_bad(dep, upstream, ready, root_phases)) {
        push_ready_failure(dep, out);
        return;
    }
    if str_field(dep, "upstream_commit") != str_field(upstream, "current_commit") {
        push_failure(dep, out);
        return;
    }
    if dependency_release_bad(lane, dep, root_phases) {
        out.push(Failure::new(
            "lane-dependency-gating",
            "dependency_release_missing",
            str_field(lane, "id"),
        ));
    }
}

fn basic_dependency_bad(dep: &Value, upstream: &Value, root_phases: &Value) -> bool {
    !upstream_status_consumable(upstream, root_phases)
        || !array_strings(upstream, "claim_ids").contains(&str_field(dep, "claim_id"))
        || str_field(dep, "validated_status") != str_field(dep, "required_status")
        || registry_ref_bad(dep, upstream)
}

fn registry_ref_bad(dep: &Value, upstream: &Value) -> bool {
    let ready_digest = dep
        .pointer("/upstream_ready_receipt/digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    ready_digest == digest::ZERO
        || str_field(dep, "evidence_digest") == digest::ZERO
        || str_field(dep, "evidence_digest") != ready_digest
        || dep
            .pointer("/upstream_ready_receipt/path")
            .and_then(Value::as_str)
            != upstream
                .pointer("/ready_receipt/path")
                .and_then(Value::as_str)
        || dep
            .pointer("/upstream_ready_receipt/digest")
            .and_then(Value::as_str)
            != upstream
                .pointer("/ready_receipt/digest")
                .and_then(Value::as_str)
}

fn ready_authority_bad(dep: &Value, upstream: &Value, ready: &Value) -> bool {
    ready.get("ready").and_then(Value::as_bool) != Some(true)
        || str_field(ready, "lane_id") != str_field(upstream, "id")
        || str_field(ready, "commit") != str_field(upstream, "current_commit")
        || str_field(ready, "branch") != str_field(upstream, "branch")
        || str_field(ready, "workspace") != str_field(upstream, "workspace")
        || str_field(ready, "base_commit") != str_field(upstream, "base_commit")
        || !array_strings(ready, "claim_ids").contains(&str_field(dep, "claim_id"))
        || ready
            .get("blocked_reasons")
            .and_then(Value::as_array)
            .is_some_and(|rows| !rows.is_empty())
        || ready.get("worktree_clean").and_then(Value::as_bool) != Some(true)
        || ready.get("teardown_ready").and_then(Value::as_bool) != Some(true)
        || canonical_digest(ready).unwrap_or_default()
            != dep
                .pointer("/upstream_ready_receipt/digest")
                .and_then(Value::as_str)
                .unwrap_or("")
}

pub(crate) fn dependency_order_bad(
    dep: &Value,
    upstream: &Value,
    ready: &Value,
    root_phases: &Value,
) -> bool {
    let Some(validated_at) =
        crate::audit::clock::parse_iso_seconds(&str_field(dep, "validated_at"))
    else {
        return true;
    };
    let Some(heartbeat_at) =
        crate::audit::clock::parse_iso_seconds(&str_field(upstream, "last_heartbeat"))
    else {
        return true;
    };
    let Some(ready_at) = crate::audit::clock::parse_iso_seconds(&str_field(ready, "generated_at"))
    else {
        return true;
    };
    let Some(post_at) = crate::audit::clock::parse_iso_seconds(&str_field(
        &root_phases["post_merge_integration_gate"],
        "validated_at",
    )) else {
        return true;
    };
    validated_at < heartbeat_at
        || validated_at < ready_at
        || validated_at < post_at
        || post_at < heartbeat_at
        || post_at < ready_at
}

fn upstream_status_consumable(upstream: &Value, root_phases: &Value) -> bool {
    str_field(upstream, "status") == "merged"
        && root_phases
            .pointer("/post_merge_integration_gate/status")
            .and_then(Value::as_str)
            == Some("pass")
}

pub(crate) fn dependency_release_bad(lane: &Value, dep: &Value, root_phases: &Value) -> bool {
    let release = &lane["dependency_release"];
    if !release.is_object() {
        return true;
    }
    let action = str_field(release, "action");
    if !["launch", "resume", "reblock"].contains(&action.as_str()) {
        return true;
    }
    let post = &root_phases["post_merge_integration_gate"];
    let post_path = post
        .pointer("/artifact_receipt/path")
        .and_then(Value::as_str)
        .unwrap_or("");
    let post_digest = post
        .pointer("/artifact_receipt/digest")
        .and_then(Value::as_str)
        .unwrap_or("");
    if str_field(release, "downstream_lane_id") != str_field(lane, "id")
        || str_field(release, "upstream_lane_id") != str_field(dep, "upstream_lane_id")
        || str_field(release, "claim_id") != str_field(dep, "claim_id")
        || release
            .pointer("/post_merge_receipt/path")
            .and_then(Value::as_str)
            .unwrap_or("")
            != post_path
        || release
            .pointer("/post_merge_receipt/digest")
            .and_then(Value::as_str)
            .unwrap_or("")
            != post_digest
    {
        return true;
    }
    if action == "reblock" && reblock_proof_bad(release, lane) {
        return true;
    }
    let Some(decided_at) =
        crate::audit::clock::parse_iso_seconds(&str_field(release, "decided_at"))
    else {
        return true;
    };
    let Some(validated_at) =
        crate::audit::clock::parse_iso_seconds(&str_field(post, "validated_at"))
    else {
        return true;
    };
    decided_at < validated_at
}

fn reblock_proof_bad(release: &Value, lane: &Value) -> bool {
    let blocker = &release["blocker"];
    let blocker_id = str_field(blocker, "id");
    blocker_id.is_empty()
        || str_field(blocker, "owner").is_empty()
        || str_field(blocker, "reason").len() < 20
        || !array_strings(blocker, "affected_claim_ids").contains(&str_field(release, "claim_id"))
        || !array_strings(lane, "blocked_reasons").contains(&blocker_id)
}

fn push_failure(dep: &Value, out: &mut Vec<Failure>) {
    out.push(Failure::new(
        "lane-dependency-gating",
        "dependency_claim_not_proven",
        str_field(dep, "claim_id"),
    ));
}

fn push_ready_failure(dep: &Value, out: &mut Vec<Failure>) {
    out.push(Failure::new(
        "ready-receipt-provenance",
        "ready_receipt_not_lane_bound",
        str_field(dep, "claim_id"),
    ));
}
