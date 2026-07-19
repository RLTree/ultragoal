use crate::context::ReadSession;
use crate::inventory::types::InventoryError;
use serde_json::Value;
use std::path::Path;
use std::process::Command;

pub(super) fn observed(registry: &Value) -> Result<(&str, &str), InventoryError> {
    gate_base(registry, true)
}

fn gate_base(registry: &Value, bind_operation: bool) -> Result<(&str, &str), InventoryError> {
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
            || gate.get("evidence_status").and_then(Value::as_str) != Some("current")
            || gate.get("observation_scope").and_then(Value::as_str)
                != Some("source_base_only_not_containing_lease_authority")
        {
            return Err(invalid("required source authority gate is not current"));
        }
        let observed = gate
            .get("observed_source_base")
            .ok_or_else(|| invalid("lease issuance gate lacks observed source base"))?;
        let current = (
            text(observed, "commit", "observed source base lacks commit")?,
            text(observed, "tree", "observed source base lacks tree")?,
        );
        let operation = text(
            gate,
            "operation_id",
            "prelaunch gate lacks operation identity",
        )?;
        let expected_operation = format!("prelaunch-{gate_id}-{}-{}", current.0, current.1);
        if (bind_operation && operation != expected_operation)
            || gate.get("candidate_ref").and_then(Value::as_str) != Some("current_clean_head")
            || gate.get("candidate_selector").and_then(Value::as_str) != Some("prelaunch_candidate")
        {
            return Err(invalid(
                "prelaunch gate operation identity does not bind source base",
            ));
        }
        if base.is_some_and(|value| value != current) {
            return Err(invalid("lease issuance gates disagree on source base"));
        }
        base = Some(current);
    }
    let base = base.ok_or_else(|| invalid("lease issuance source base is unavailable"))?;
    Ok(base)
}

pub(super) fn validate_git(
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
    let committed = git(root, &["show", &format!("{head}:LANE_REGISTRY.json")])?;
    let committed_registry: Value = serde_json::from_str(&committed)
        .map_err(|_| invalid("committed lane registry is malformed"))?;
    if base != gate_base(&committed_registry, false)? {
        return Err(invalid("lease source base is not the root-issued base"));
    }
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

fn text<'a>(value: &'a Value, field: &str, message: &str) -> Result<&'a str, InventoryError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(message))
}

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}
