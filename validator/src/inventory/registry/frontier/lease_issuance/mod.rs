use super::{SchedulerNodes, change_impact, handoff_adjacency, lease_base};
use crate::context::ReadSession;
use crate::inventory::types::InventoryError;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

mod debt_worktree;
mod worktree_identity;

pub(super) fn validate(
    reads: &ReadSession,
    root: &Path,
    registry: &Value,
    nodes: &SchedulerNodes,
) -> Result<(), InventoryError> {
    let active = registry
        .pointer("/lease_state/status")
        .and_then(Value::as_str)
        == Some("active");
    if !active && !nodes.active_worktree_lanes.is_empty() {
        return Err(invalid("active worktree lifecycle lacks an active lease"));
    }
    if !active {
        return Ok(());
    }
    let base = lease_base::observed(registry)?;
    lease_base::validate_git(reads, root, base)?;
    let records = registry
        .pointer("/lease_state/active_records")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("active lease records are missing"))?;
    let lanes = registry
        .get("lanes")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("scheduler lanes are missing"))?;
    let scopes = registry
        .get("scope_mappings")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("scope mappings are missing"))?;
    reject_terminal_p0(records)?;
    debt_worktree::validate(records, registry, base, root)?;
    let scheduler_records = records
        .iter()
        .filter(|record| !debt_worktree::is_record(record))
        .cloned()
        .collect::<Vec<_>>();
    active_record_lanes(&scheduler_records, &nodes.active_worktree_lanes)?;
    for record in &scheduler_records {
        let lane_id = text(record, "lane_id", "active lease lacks lane ID")?;
        if text(record, "base_commit", "active lease lacks base commit")? != base.0
            || text(record, "base_tree", "active lease lacks base tree")? != base.1
        {
            return Err(invalid("active lease base differs from source base"));
        }
        worktree_identity::validate(root, registry, record, base)?;
        let lane = find(lanes, "id", lane_id, "active lease names an unknown lane")?;
        exact_field(
            record,
            lane,
            "owner",
            "active lease owner differs from lane",
        )?;
        exact_field(
            record,
            lane,
            "scope_ids",
            "active lease scopes differ from lane",
        )?;
        validate_dependencies(record, lane, lanes)?;
        let scope_id = lane
            .get("scope_ids")
            .and_then(Value::as_array)
            .and_then(|ids| (ids.len() == 1).then(|| ids[0].as_str()).flatten())
            .ok_or_else(|| invalid("active lease lane must name one scope"))?;
        let scope = find(
            scopes,
            "scope_id",
            scope_id,
            "active lease scope is missing",
        )?;
        for (record_field, scope_field) in [
            ("owned_files", "owned_roots"),
            ("owned_symbols", "owned_symbols"),
            ("generated_outputs", "generated_roots"),
            ("fixtures", "fixture_roots"),
            ("effects", "effects"),
        ] {
            exact_named(record, record_field, scope, scope_field)?;
        }
        change_impact::validate(reads, registry, lanes, scopes, record, lane, base)?;
        handoff_adjacency::validate(reads, root, record)?;
    }
    Ok(())
}

fn reject_terminal_p0(records: &[Value]) -> Result<(), InventoryError> {
    if records.iter().any(debt_worktree::is_record) {
        return Err(invalid("completed P0 repair cannot be reissued"));
    }
    Ok(())
}

pub(super) fn active_record_lanes(
    records: &[Value],
    active_worktree_lanes: &BTreeSet<String>,
) -> Result<BTreeSet<String>, InventoryError> {
    let mut seen = BTreeSet::new();
    for record in records {
        let lane_id = text(record, "lane_id", "active lease lacks lane ID")?;
        if !active_worktree_lanes.contains(lane_id) || !seen.insert(lane_id.to_owned()) {
            return Err(invalid(
                "active leases do not exactly match active worktree lanes",
            ));
        }
        if !matches!(
            record.get("status").and_then(Value::as_str),
            Some("issued" | "ready")
        ) {
            return Err(invalid("active lease references a closed worktree"));
        }
        if text(record, "worktree", "active lease lacks worktree")?.is_empty() {
            return Err(invalid("active lease lacks worktree"));
        }
    }
    if &seen != active_worktree_lanes {
        return Err(invalid(
            "active leases do not exactly match active worktree lanes",
        ));
    }
    Ok(seen)
}

fn validate_dependencies(
    record: &Value,
    lane: &Value,
    lanes: &[Value],
) -> Result<(), InventoryError> {
    let dependencies = lane
        .get("dependencies")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("lane dependencies are missing"))?;
    let consumed = record
        .pointer("/consumed_set/dependency_identities")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("lease dependency identities are missing"))?;
    let expected = dependencies
        .iter()
        .map(|id| {
            let id = id
                .as_str()
                .ok_or_else(|| invalid("lane dependency ID is malformed"))?;
            find(lanes, "id", id, "lane dependency is missing")?
                .get("current_identity")
                .cloned()
                .ok_or_else(|| invalid("lane dependency identity is missing"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if consumed != &expected {
        return Err(invalid("lease dependency identities are stale"));
    }
    Ok(())
}

fn exact_field(
    left: &Value,
    right: &Value,
    field: &str,
    message: &str,
) -> Result<(), InventoryError> {
    if left.get(field) != right.get(field) {
        return Err(invalid(message));
    }
    Ok(())
}

fn exact_named(
    left: &Value,
    left_field: &str,
    right: &Value,
    right_field: &str,
) -> Result<(), InventoryError> {
    if left.get(left_field) != right.get(right_field) {
        return Err(invalid(&format!(
            "lease {left_field} differs from scope authority"
        )));
    }
    Ok(())
}

fn find<'a>(
    rows: &'a [Value],
    field: &str,
    value: &str,
    message: &str,
) -> Result<&'a Value, InventoryError> {
    rows.iter()
        .find(|row| row.get(field).and_then(Value::as_str) == Some(value))
        .ok_or_else(|| invalid(message))
}

fn text<'a>(value: &'a Value, field: &str, message: &str) -> Result<&'a str, InventoryError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(message))
}

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}

#[cfg(test)]
#[test]
fn completed_p0_cannot_reenter_active_lease_records() {
    let records = [serde_json::json!({"exception_id":"P0-DEBT-REPAIR"})];
    assert!(reject_terminal_p0(&records).is_err());
}
