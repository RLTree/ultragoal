mod debt;
mod eligibility;
mod overlap;
mod plan_binding;
mod record;
mod root;
#[cfg(test)]
mod tests;

pub(super) fn protected_path_match(normalized: &str, pattern: &str) -> bool {
    let base = pattern.strip_suffix("/**").unwrap_or(pattern);
    if base.starts_with('/') {
        normalized == base || normalized.starts_with(&format!("{base}/"))
    } else {
        let suffix = format!("/{base}");
        normalized == base
            || normalized.ends_with(&suffix)
            || normalized.contains(&format!("{suffix}/"))
    }
}

use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn check(registry: &Value, root: &Path, out: &mut Vec<Failure>) {
    let state = &registry["lease_state"];
    let rows = state
        .get("active_records")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let status = str_field(state, "status");
    if (status == "unissued" && !rows.is_empty()) || (status == "active" && rows.is_empty()) {
        out.push(Failure::new(
            "authority-lease",
            "lease_state_records_mismatch",
            "lease_state",
        ));
    }
    if rows.len() > 4 {
        out.push(Failure::new(
            "authority-lease",
            "lease_concurrency_limit",
            rows.len().to_string(),
        ));
    }
    let p0_count = rows
        .iter()
        .filter(|row| str_field(row, "exception_id") == "P0-DEBT-REPAIR")
        .count();
    if p0_count > 0 && (p0_count != 1 || rows.len() != 1) {
        out.push(Failure::new(
            "authority-lease",
            "p0_lease_must_be_serial_root_only",
            "active_records",
        ));
    }
    for row in &rows {
        record::validate_record(row, registry, root, out);
        eligibility::check(row, registry, out);
        check_consumed_binding(row, registry, root, out);
    }
    eligibility::check_gate_config(registry, out);
    for (index, left) in rows.iter().enumerate() {
        for right in rows.iter().skip(index + 1) {
            overlap::compare_records(
                left,
                right,
                registry["lease_state"]
                    .get("worktree_root")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                out,
            );
        }
    }
    let Some(template) = state.get("record_template") else {
        out.push(Failure::new(
            "authority-lease",
            "lease_template_missing",
            "record_template",
        ));
        return;
    };
    record::validate_record(template, registry, root, out);
}

fn check_consumed_binding(record: &Value, registry: &Value, root: &Path, out: &mut Vec<Failure>) {
    if str_field(record, "status") == "unissued"
        || str_field(record, "exception_id") == "P0-DEBT-REPAIR"
    {
        return;
    }
    let lane_id = str_field(record, "lane_id");
    let Some(lane) = registry["lanes"]
        .as_array()
        .and_then(|lanes| lanes.iter().find(|lane| str_field(lane, "id") == lane_id))
    else {
        return;
    };
    let expected = lane
        .pointer("/consumption_contract/dependency_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let invalidated = overlap::array_set(record, "invalidated_by");
    let identities = record
        .pointer("/consumed_set/dependency_identities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut actual = BTreeSet::new();
    let mut stale_identity = false;
    for identity in &identities {
        let id = str_field(identity, "lane_id");
        if id.is_empty() || !actual.insert(id.clone()) {
            stale_identity = true;
            continue;
        }
        let current = registry["lanes"]
            .as_array()
            .and_then(|lanes| lanes.iter().find(|lane| str_field(lane, "id") == id))
            .and_then(|lane| lane.get("current_identity"));
        if current.is_none_or(|current| current != identity) {
            stale_identity = true;
        }
    }
    if actual != expected || invalidated != expected {
        out.push(Failure::new(
            "authority-lease",
            "lease_consumed_dependency_contract_mismatch",
            lane_id.clone(),
        ));
    }
    let surfaces = [
        "files",
        "symbols",
        "generated_outputs",
        "fixtures",
        "effects",
    ];
    let surface_count = surfaces
        .iter()
        .map(|key| overlap::array_set(&record["consumed_set"], key).len())
        .sum::<usize>();
    if !expected.is_empty() && surface_count == 0 {
        out.push(Failure::new(
            "authority-lease",
            "lease_consumed_surface_missing",
            lane_id.clone(),
        ));
    }
    let refresh = &record["refresh_state"];
    let (candidate, _, _) = root::current_candidate(root);
    let observed_commit = str_field(refresh, "observed_root_commit");
    let observed_tree = str_field(refresh, "observed_root_tree");
    let observation_is_valid = root::is_ancestor(root, &observed_commit, &candidate)
        && root::tree_at(root, &observed_commit) == observed_tree;
    let changed = root::changed_paths(root, &observed_commit, &candidate);
    let touched = ["files", "generated_outputs", "fixtures"]
        .into_iter()
        .flat_map(|key| overlap::array_set(&record["consumed_set"], key))
        .any(|path| changed.contains(&path));
    let invalidated = stale_identity || touched;
    let status = str_field(refresh, "status");
    if !observation_is_valid
        || (invalidated && status != "invalidated")
        || (!invalidated && status != "current")
    {
        out.push(Failure::new(
            "authority-lease",
            "lease_refresh_state_mismatch",
            lane_id,
        ));
    }
    if invalidated && matches!(str_field(record, "status").as_str(), "ready" | "closing") {
        out.push(Failure::new(
            "authority-lease",
            "invalidated_lease_cannot_handoff",
            "status",
        ));
    }
}
