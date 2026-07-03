use crate::audit::contract::Failure;
use crate::claim_semantics::{array_strings, lane::paths, path_contains, str_field};
use crate::digest;
use serde_json::{Value, json};
use std::path::Path;

pub(crate) fn lane_overlap(lanes: &[Value], out: &mut Vec<Failure>) {
    let mut seen: Vec<String> = Vec::new();
    for lane in lanes {
        if crate::claim_semantics::lane::status::blocks_isolation(lane) {
            for owned in array_strings(lane, "owned_paths") {
                let owned = match paths::normalize_owned(&owned) {
                    Ok(path) => path,
                    Err(err) => {
                        out.push(Failure::new(
                            "lane-scope-overlap",
                            "invalid_lane_owned_path",
                            format!("{owned}: {err}"),
                        ));
                        continue;
                    }
                };
                if seen.iter().any(|prior| {
                    paths::contains_owned(prior, &owned).unwrap_or(false)
                        || paths::contains_owned(&owned, prior).unwrap_or(false)
                }) {
                    out.push(Failure::new(
                        "lane-scope-overlap",
                        "active_lane_owned_path_overlap",
                        owned.clone(),
                    ));
                }
                seen.push(owned);
            }
        }
    }
}

pub(crate) fn root_verification_states(
    states: &Value,
    lanes: &[Value],
    root: &Path,
    out: &mut Vec<Failure>,
) {
    let pre = &states["pre_merge_lane_gate"];
    let post = &states["post_merge_integration_gate"];
    let final_all_lanes_state = &states["final_all_lanes_gate"];
    if str_field(post, "status") == "pass" && str_field(pre, "status") != "pass" {
        out.push(Failure::new(
            "root-verification-stage-coverage",
            "post_merge_without_pre_merge_receipt",
            "post gate",
        ));
    }
    if str_field(final_all_lanes_state, "status") == "pass" && str_field(post, "status") != "pass" {
        out.push(Failure::new(
            "root-verification-stage-coverage",
            "final_gate_without_post_merge_receipt",
            "final gate",
        ));
    }
    final_all_lanes_state_open_lanes(final_all_lanes_state, lanes, out);
    for state in states.as_object().into_iter().flat_map(|obj| obj.values()) {
        if str_field(state, "status") == "pass" && verification_receipt_bad(root, state) {
            out.push(Failure::new(
                "root-verification-stage-coverage",
                "root_verification_stage_pass_without_successful_receipt",
                str_field(state, "stage"),
            ));
        }
    }
    post_merge_receipt_check(root, post, lanes, out);
}

pub(crate) fn parent_changed_files_are_registered(
    lanes: &[Value],
    ready: &Value,
    out: &mut Vec<Failure>,
) {
    let owned = lanes
        .iter()
        .flat_map(|lane| array_strings(lane, "owned_paths"))
        .collect::<Vec<_>>();
    if array_strings(ready, "changed_files")
        .iter()
        .any(|path| !owned.iter().any(|owned| path_contains(owned, path)))
    {
        out.push(Failure::new(
            "parent-role-boundary",
            "parent_diff_without_registered_lane",
            array_strings(ready, "changed_files").join(","),
        ));
    }
}

fn final_all_lanes_state_open_lanes(
    final_all_lanes_state: &Value,
    lanes: &[Value],
    out: &mut Vec<Failure>,
) {
    if str_field(final_all_lanes_state, "status") != "pass" {
        return;
    }
    let open = lanes
        .iter()
        .filter(|lane| {
            !crate::claim_semantics::lane::status::is_terminal(&str_field(lane, "status"))
        })
        .map(|lane| str_field(lane, "id"))
        .collect::<Vec<_>>();
    if !open.is_empty() {
        out.push(Failure::new(
            "root-verification-stage-coverage",
            "final_gate_with_nonterminal_lanes",
            open.join(","),
        ));
    }
}

fn verification_receipt_bad(root: &Path, state: &Value) -> bool {
    let command = &state["command_receipt"];
    let artifact = &state["artifact_receipt"];
    let command_as_ref = json!({
        "path": command.pointer("/artifact_path").and_then(Value::as_str).unwrap_or(""),
        "digest": command.pointer("/artifact_digest").and_then(Value::as_str).unwrap_or("")
    });
    state
        .pointer("/command_receipt/exit")
        .and_then(Value::as_i64)
        != Some(0)
        || state
            .pointer("/artifact_receipt/digest")
            .and_then(Value::as_str)
            .is_none_or(|d| d == digest::ZERO)
        || state
            .pointer("/command_receipt/artifact_path")
            .and_then(Value::as_str)
            != state
                .pointer("/artifact_receipt/path")
                .and_then(Value::as_str)
        || state
            .pointer("/command_receipt/artifact_digest")
            .and_then(Value::as_str)
            != state
                .pointer("/artifact_receipt/digest")
                .and_then(Value::as_str)
        || crate::package::artifact::refs::validate_object(
            root,
            artifact,
            "root verification stage artifact",
        )
        .is_err()
        || crate::package::artifact::refs::validate_object(
            root,
            &command_as_ref,
            "root command artifact",
        )
        .is_err()
}

fn post_merge_receipt_check(root: &Path, state: &Value, lanes: &[Value], out: &mut Vec<Failure>) {
    if str_field(state, "status") != "pass" {
        return;
    }
    let path = str_field(&state["artifact_receipt"], "path");
    let Ok(receipt) = crate::json_boundary::read_json(&root.join(path)) else {
        out.push(post_merge_failure("post_merge_receipt_not_lane_bound"));
        return;
    };
    let lane_id = str_field(&receipt, "upstream_lane_id");
    let lane = lanes.iter().find(|lane| str_field(lane, "id") == lane_id);
    if post_merge_receipt_bad(state, &receipt, lane) {
        out.push(post_merge_failure("post_merge_receipt_not_lane_bound"));
    }
}

fn post_merge_receipt_bad(state: &Value, receipt: &Value, lane: Option<&Value>) -> bool {
    let Some(lane) = lane else {
        return true;
    };
    let command_ok = receipt
        .get("commands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|row| {
            str_field(row, "id") == str_field(&state["command_receipt"], "id")
                && row.get("exit").and_then(Value::as_i64) == Some(0)
        });
    str_field(receipt, "schema") != "harness-ultragoal.root-verification-receipt.v1"
        || str_field(receipt, "stage") != "post_merge_integration_gate"
        || str_field(receipt, "upstream_commit") != str_field(lane, "current_commit")
        || str_field(receipt, "target_branch") != str_field(lane, "target_branch")
        || str_field(receipt, "validated_at") != str_field(state, "validated_at")
        || receipt
            .pointer("/merge_reachability/upstream_commit_reachable")
            .and_then(Value::as_bool)
            != Some(true)
        || !command_ok
        || crate::audit::clock::parse_iso_seconds(&str_field(receipt, "validated_at")).is_none()
}

fn post_merge_failure(error: &str) -> Failure {
    Failure::new(
        "root-verification-stage-coverage",
        error,
        "post_merge_integration_gate",
    )
}
