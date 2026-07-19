use crate::inventory::types::InventoryError;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

const EXCEPTION: &str = "P0-DEBT-REPAIR";

mod diagnostic_source;
mod live_worktree;

pub(super) fn is_record(record: &Value) -> bool {
    record.get("exception_id").and_then(Value::as_str) == Some(EXCEPTION)
}

pub(super) fn validate(
    records: &[Value],
    registry: &Value,
    base: (&str, &str),
    root: &Path,
) -> Result<(), InventoryError> {
    let debt = records.iter().filter(|record| is_record(record));
    let allowed = strings(
        registry.pointer("/lease_state/p0_exception/allowed/paths"),
        "P0 allowed paths are missing",
    )?;
    let forbidden = strings(
        registry.pointer("/lease_state/p0_exception/forbidden/paths"),
        "P0 forbidden paths are missing",
    )?;
    let mut lease_ids = BTreeSet::new();
    let mut branches = BTreeSet::new();
    let mut worktrees = BTreeSet::new();
    let mut claimed = BTreeSet::new();
    let mut count = 0;
    for record in debt {
        count += 1;
        exact(record, "lane_id", "P0", "P0 lease has the wrong lane ID")?;
        exact(
            record,
            "base_commit",
            base.0,
            "P0 lease base differs from source base",
        )?;
        exact(
            record,
            "base_tree",
            base.1,
            "P0 lease tree differs from source base",
        )?;
        if !matches!(
            record.get("status").and_then(Value::as_str),
            Some("issued" | "ready")
        ) {
            return Err(invalid("P0 lease is not active"));
        }
        unique(record, "lease_id", &mut lease_ids)?;
        unique(record, "branch", &mut branches)?;
        unique(record, "worktree", &mut worktrees)?;
        live_worktree::validate(root, registry, record, base)?;
        let diagnostic = strings(
            record.get("diagnostic_paths"),
            "P0 diagnostic paths are missing",
        )?;
        if diagnostic.is_empty() {
            return Err(invalid("P0 diagnostic paths are empty"));
        }
        let dependency_paths = strings(
            record.get("support_files"),
            "P0 dependency paths are missing",
        )?;
        let owned = strings(record.get("owned_files"), "P0 owned files are missing")?;
        let expected = diagnostic
            .union(&dependency_paths)
            .cloned()
            .collect::<BTreeSet<_>>();
        if owned != expected {
            return Err(invalid(
                "P0 owned files differ from diagnostic and support union",
            ));
        }
        if digest(&diagnostic)
            != text(
                record,
                "diagnostic_path_set_digest",
                "P0 path-set digest is missing",
            )?
        {
            return Err(invalid("P0 diagnostic path-set digest is stale"));
        }
        for path in owned {
            if !allowed.contains(&path) {
                return Err(invalid("P0 lease owns a path outside the exact allowlist"));
            }
            if forbidden.iter().any(|pattern| matches_path(&path, pattern)) {
                return Err(invalid("P0 lease owns a forbidden path"));
            }
            if !claimed.insert(path) {
                return Err(invalid("P0 worktree leases overlap"));
            }
        }
    }
    if count > 0 {
        validate_product_dag_is_blocked(records, registry)?;
        diagnostic_source::validate(records, registry, base)?;
        if claimed != allowed {
            return Err(invalid(
                "P0 worktree leases do not exactly cover the allowed path set",
            ));
        }
    }
    Ok(())
}

fn validate_product_dag_is_blocked(
    records: &[Value],
    registry: &Value,
) -> Result<(), InventoryError> {
    if records.iter().any(|record| !is_record(record)) {
        return Err(invalid("P0 worktrees overlap an active product lease"));
    }
    let eligible = registry
        .pointer("/pre_adoption_source/eligible_scheduler_nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("P0 worktrees lack scheduler eligibility state"))?;
    if !eligible.is_empty() {
        return Err(invalid("P0 worktrees require a blocked product DAG"));
    }
    let lanes = registry
        .get("lanes")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("P0 worktrees lack product lane state"))?;
    let active = lanes.iter().any(|lane| {
        matches!(
            lane.get("state").and_then(Value::as_str),
            Some(
                "ready"
                    | "leased"
                    | "candidate"
                    | "under_review"
                    | "rework"
                    | "accepted"
                    | "integrating"
            )
        )
    });
    if active {
        return Err(invalid(
            "P0 worktrees require every product lane to be idle",
        ));
    }
    Ok(())
}

fn strings(value: Option<&Value>, message: &str) -> Result<BTreeSet<String>, InventoryError> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| invalid(message))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| invalid(message))
        })
        .collect()
}

fn unique(record: &Value, field: &str, seen: &mut BTreeSet<String>) -> Result<(), InventoryError> {
    let value = text(record, field, "P0 lease identity is missing")?;
    if value.is_empty() || !seen.insert(value.to_owned()) {
        return Err(invalid("P0 lease identity is missing or duplicated"));
    }
    Ok(())
}

fn exact(record: &Value, field: &str, expected: &str, message: &str) -> Result<(), InventoryError> {
    if text(record, field, message)? != expected {
        return Err(invalid(message));
    }
    Ok(())
}

fn text<'a>(record: &'a Value, field: &str, message: &str) -> Result<&'a str, InventoryError> {
    record
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(message))
}

fn digest(paths: &BTreeSet<String>) -> String {
    let bytes = paths
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    format!("sha256:{:x}", Sha256::digest(bytes.as_bytes()))
}

fn matches_path(path: &str, pattern: &str) -> bool {
    pattern
        .strip_suffix("/**")
        .is_some_and(|root| path == root || path.starts_with(&format!("{root}/")))
        || path == pattern
}

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}

#[cfg(test)]
mod test_fixture;
#[cfg(test)]
mod tests;
