use super::*;
use std::process::Command;

pub(super) fn validate(
    reads: &ReadSession,
    root: &Path,
    registry: &Value,
    ready: &BTreeSet<String>,
) -> Result<(), InventoryError> {
    if registry
        .pointer("/lease_state/status")
        .and_then(Value::as_str)
        != Some("active")
    {
        return Ok(());
    }
    let base = observed_base(registry)?;
    validate_git_base(reads, root, base)?;
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
    let mut seen = BTreeSet::new();
    for record in records {
        let lane_id = text(record, "lane_id", "active lease lacks lane ID")?;
        if !ready.contains(lane_id) || !seen.insert(lane_id.to_owned()) {
            return Err(invalid("active leases do not exactly match ready lanes"));
        }
        if text(record, "base_commit", "active lease lacks base commit")? != base.0
            || text(record, "base_tree", "active lease lacks base tree")? != base.1
        {
            return Err(invalid("active lease base differs from source base"));
        }
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
    }
    if &seen != ready {
        return Err(invalid("active leases do not exactly match ready lanes"));
    }
    Ok(())
}

fn observed_base(registry: &Value) -> Result<(&str, &str), InventoryError> {
    let gates = registry
        .get("prelaunch_gates")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("prelaunch gates are missing"))?;
    let mut base = None;
    for gate_id in ["compile", "namespace", "standards"] {
        let matches = gates
            .iter()
            .filter(|gate| gate.get("id").and_then(Value::as_str) == Some(gate_id))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(invalid("required gate is missing or duplicated"));
        }
        let gate = matches[0];
        if gate.get("status").and_then(Value::as_str) != Some("current")
            || gate
                .get(concat!("evidence", "_status"))
                .and_then(Value::as_str)
                != Some("current")
            || gate.get("observation_scope").and_then(Value::as_str)
                != Some("source_base_only_not_containing_lease_authority")
        {
            return Err(invalid("required lease issuance gate is not current"));
        }
        let observed = gate
            .get("observed_source_base")
            .ok_or_else(|| invalid("lease issuance gate lacks observed source base"))?;
        let current = (
            text(observed, "commit", "observed source base lacks commit")?,
            text(observed, "tree", "observed source base lacks tree")?,
        );
        if base.is_some_and(|value| value != current) {
            return Err(invalid("lease issuance gates disagree on source base"));
        }
        base = Some(current);
    }
    base.ok_or_else(|| invalid("lease issuance source base is unavailable"))
}

fn validate_git_base(
    reads: &ReadSession,
    root: &Path,
    base: (&str, &str),
) -> Result<(), InventoryError> {
    let tree = git(root, &["rev-parse", &format!("{}^{{tree}}", base.0)])?;
    if tree != base.1 {
        return Err(invalid("observed source base has the wrong Git tree"));
    }
    let head = reads
        .context
        .candidate()
        .head_commit
        .as_deref()
        .ok_or_else(|| invalid("current authority commit is unavailable"))?;
    let status = Command::new("git")
        .args([
            "-C",
            root.to_string_lossy().as_ref(),
            "merge-base",
            "--is-ancestor",
            base.0,
            head,
        ])
        .status()
        .map_err(|error| invalid(&format!("cannot validate lease base ancestry: {error}")))?;
    if !status.success() {
        return Err(invalid("source base is not a current authority ancestor"));
    }
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<String, InventoryError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| invalid(&format!("cannot inspect lease Git identity: {error}")))?;
    if !output.status.success() {
        return Err(invalid("cannot inspect lease Git identity"));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| invalid("lease Git identity is not UTF-8"))
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
