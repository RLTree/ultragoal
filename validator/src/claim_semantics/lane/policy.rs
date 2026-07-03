use crate::audit::contract::Failure;
use crate::claim_semantics::{array_strings, str_field};
use serde_json::Value;
use std::collections::BTreeMap;

pub fn validation_clock(bundle: &Value, out: &mut Vec<Failure>) -> i64 {
    let raw = bundle
        .pointer("/automation_tick_receipt/freshness_policy/clock_at_validation")
        .and_then(Value::as_str)
        .or_else(|| {
            bundle
                .pointer("/automation_tick_receipt/checked_at")
                .and_then(Value::as_str)
        })
        .unwrap_or("");
    match crate::audit::clock::parse_iso_seconds(raw) {
        Some(value) => value,
        None => {
            out.push(Failure::new(
                "automation-tick-freshness",
                "validation_clock_timestamp_malformed",
                "automation_tick_receipt",
            ));
            0
        }
    }
}

pub fn check_lanes(
    bundle: &Value,
    lr: &Value,
    ready: &Value,
    ready_receipts: &[Value],
    root: &std::path::Path,
    run_at: i64,
    out: &mut Vec<Failure>,
) {
    let lanes = lr
        .get("lanes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let lane_index = lanes
        .iter()
        .map(|lane| (str_field(lane, "id"), lane))
        .collect::<BTreeMap<_, _>>();
    let ready_index = ready_receipts
        .iter()
        .map(|receipt| (str_field(receipt, "lane_id"), receipt))
        .collect::<BTreeMap<_, _>>();
    for lane in &lanes {
        lane_check(
            lane,
            &lane_index,
            &ready_index,
            &lr["root_verification_stages"],
            bundle,
            root,
            run_at,
            out,
        );
    }
    crate::claim_semantics::lane::root::scope::lane_overlap(&lanes, out);
    crate::claim_semantics::lane::isolation::mutable_resources(&lanes, out);
    crate::claim_semantics::lane::root::scope::root_verification_states(
        &lr["root_verification_stages"],
        &lanes,
        root,
        out,
    );
    crate::claim_semantics::lane::root::scope::parent_changed_files_are_registered(
        &lanes, ready, out,
    );
}

fn lane_check(
    lane: &Value,
    lane_index: &BTreeMap<String, &Value>,
    ready_index: &BTreeMap<String, &Value>,
    root_verification_states: &Value,
    bundle: &Value,
    root: &std::path::Path,
    run_at: i64,
    out: &mut Vec<Failure>,
) {
    lane_size(lane, out);
    if let Some(error) =
        crate::claim_semantics::lane::actor::freshness::actor_freshness_error(lane, run_at)
    {
        out.push(Failure::new(
            "lane-actor-binding",
            &error,
            str_field(lane, "id"),
        ));
    }
    crate::claim_semantics::lane::runtime::state::target_freshness(lane, out);
    crate::claim_semantics::lane::runtime::state::worktree_status(lane, out);
    for dep in lane
        .get("dependencies")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        crate::claim_semantics::lane::dependency::check(
            lane,
            dep,
            lane_index,
            ready_index,
            root_verification_states,
            bundle,
            root,
            out,
        );
    }
    crate::claim_semantics::lane::runtime::state::teardown(lane, root, out);
}

fn lane_size(lane: &Value, out: &mut Vec<Failure>) {
    let ev = &lane["lane_size_evidence"];
    if array_strings(ev, "owned_path_groups").len() < 3
        || array_strings(ev, "work_units").len() < 4
        || str_field(ev, "independent_outcome").len() < 80
        || str_field(ev, "why_not_parent_inline").len() < 30
        || str_field(ev, "why_not_smaller").len() < 30
    {
        out.push(Failure::new(
            "macro-lane-sizing",
            "lane_too_small_or_overlapping",
            str_field(lane, "id"),
        ));
    }
}
