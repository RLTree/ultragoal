use super::{digest, invalid, strings, text};
use crate::inventory::types::InventoryError;
use serde_json::Value;
use std::collections::BTreeSet;

const SOURCES: [(&str, &str, &str); 5] = [
    (
        "compile",
        "cargo check --manifest-path validator/Cargo.toml --lib --message-format=short",
        "compile_diagnostics",
    ),
    (
        "clippy",
        "cargo clippy --locked --manifest-path validator/Cargo.toml --lib -- -D warnings",
        "clippy_diagnostics",
    ),
    (
        "namespace",
        "target/debug/ultragoal --json --root . check strict --claim namespace-progressive-disclosure",
        "namespace_findings",
    ),
    (
        "standards",
        "scripts/check-agent-standards .",
        "standards_findings",
    ),
    (
        "retention",
        "authorized retention reconciliation of validation_artifacts/review and .codex-worktree home/tmp",
        "retention_inventory",
    ),
];

pub(super) fn validate(
    records: &[Value],
    registry: &Value,
    base: (&str, &str),
) -> Result<(), InventoryError> {
    let transition = registry
        .pointer("/lease_state/p0_exception/issuance_transition")
        .ok_or_else(|| invalid("P0 diagnostic issuance transition is missing"))?;
    if transition.get("status").and_then(Value::as_str) != Some("issued") {
        return Err(invalid("P0 diagnostic sources were not issued"));
    }
    let source_sets = transition
        .get("source_sets")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("P0 diagnostic source sets are missing"))?;
    let observations = registry
        .pointer("/lease_state/p0_exception/debt_path_sources")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("P0 diagnostic observations are missing"))?;
    let mut complete_paths = BTreeSet::new();
    for (source_id, command, source_kind) in SOURCES {
        let paths = strings(
            source_sets.get(source_id),
            "P0 diagnostic source path set is missing",
        )?;
        complete_paths.extend(paths.iter().cloned());
        let observation = observations
            .get(source_id)
            .ok_or_else(|| invalid("P0 diagnostic observation is missing"))?;
        validate_observation(source_id, command, source_kind, observation, &paths, base)?;
    }
    let mut leased_paths = BTreeSet::new();
    for record in records.iter().filter(|record| super::is_record(record)) {
        let diagnostic_paths = strings(
            record.get("diagnostic_paths"),
            "P0 diagnostic paths are missing",
        )?;
        leased_paths.extend(diagnostic_paths.iter().cloned());
        let source_kind = text(
            record,
            "diagnostic_source_kind",
            "P0 diagnostic source kind is missing",
        )?;
        let (source_id, observation) = observations
            .iter()
            .find(|(_, value)| {
                value.get("source_kind").and_then(Value::as_str) == Some(source_kind)
            })
            .ok_or_else(|| invalid("P0 lease names an unknown diagnostic source"))?;
        let observed_paths = strings(
            source_sets.get(source_id),
            "P0 lease diagnostic source paths are missing",
        )?;
        if !diagnostic_paths.is_subset(&observed_paths) {
            return Err(invalid("P0 lease mislabels its diagnostic source"));
        }
        for field in ["operation_id", "tool", "observed_at"] {
            if record.get(field) != observation.get(field) {
                return Err(invalid("P0 lease diagnostic identity is stale"));
            }
        }
    }
    if leased_paths != complete_paths {
        return Err(invalid(
            "P0 leases do not exactly cover current diagnostic paths",
        ));
    }
    Ok(())
}

fn validate_observation(
    source_id: &str,
    command: &str,
    source_kind: &str,
    observation: &Value,
    paths: &BTreeSet<String>,
    base: (&str, &str),
) -> Result<(), InventoryError> {
    if observation.get("status").and_then(Value::as_str) != Some("current")
        || observation.get("command").and_then(Value::as_str) != Some(command)
        || observation.get("source_kind").and_then(Value::as_str) != Some(source_kind)
        || text(
            observation,
            "candidate_commit",
            "P0 diagnostic candidate commit is missing",
        )? != base.0
        || text(
            observation,
            "candidate_tree",
            "P0 diagnostic candidate tree is missing",
        )? != base.1
        || observation
            .get("exit_code")
            .and_then(Value::as_i64)
            .is_none()
        || observation.get("output_path") != Some(&Value::Null)
    {
        return Err(invalid(
            "P0 diagnostic observation is not current for its candidate",
        ));
    }
    let operation_id = text(
        observation,
        "operation_id",
        "P0 diagnostic operation identity is missing",
    )?;
    if operation_id != format!("p0-{source_id}-{}-{}", base.0, base.1)
        || text(observation, "tool", "P0 diagnostic tool is missing")?.is_empty()
        || text(
            observation,
            "observed_at",
            "P0 diagnostic observation time is missing",
        )?
        .is_empty()
        || text(
            observation,
            "diagnostic_path_set_digest",
            "P0 diagnostic path-set digest is missing",
        )? != digest(paths)
    {
        return Err(invalid("P0 diagnostic observation identity is stale"));
    }
    Ok(())
}
