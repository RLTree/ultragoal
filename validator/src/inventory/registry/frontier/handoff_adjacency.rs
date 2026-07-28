use crate::context::ReadSession;
use crate::inventory::types::InventoryError;
use crate::package::inventory::{inventory_paths, package_digest_excluded};
use serde_json::Value;
use std::path::Path;

pub(super) const RECEIPT: &str = "validation_artifacts/worker-results/N11-EVAL-RESEARCH.json";

pub(super) fn validate(
    reads: &ReadSession,
    root: &Path,
    record: &Value,
) -> Result<(), InventoryError> {
    if record.get("lane_id").and_then(Value::as_str) != Some("N11") {
        return unissued(record);
    }
    match record.get("status").and_then(Value::as_str) {
        Some("issued") => unissued(record),
        Some("ready") => verified(reads, root, record),
        _ => Err(invalid("lease handoff has an illegal status")),
    }
}

fn unissued(record: &Value) -> Result<(), InventoryError> {
    if record
        .pointer("/receipt_child/status")
        .and_then(Value::as_str)
        == Some("unissued")
    {
        Ok(())
    } else {
        Err(invalid(
            "lease receipt child precedes a verified source handoff",
        ))
    }
}

fn verified(reads: &ReadSession, root: &Path, record: &Value) -> Result<(), InventoryError> {
    verify(reads, root, record)?;
    reads
        .revalidate()
        .map_err(|_| invalid("lease handoff validation became stale"))
}

pub(super) fn verify(
    reads: &ReadSession,
    root: &Path,
    record: &Value,
) -> Result<(), InventoryError> {
    let source = handoff(record)?;
    let base = text(record.get("base_commit"), "lease source base is missing")?;
    if git(reads, root, &["rev-parse", &format!("{source}^{{tree}}")])?
        != text(
            record.pointer("/handoff/tree"),
            "lease handoff tree is missing",
        )?
        || git(reads, root, &["rev-parse", &format!("{source}^")])? != base
    {
        return Err(invalid("lease source handoff is not adjacent to its base"));
    }
    let child = record
        .get("receipt_child")
        .ok_or_else(|| invalid("lease receipt child is missing"))?;
    if child.get("status").and_then(Value::as_str) != Some("verified")
        || child.get("path").and_then(Value::as_str) != Some(RECEIPT)
        || text(
            child.get("source_commit"),
            "lease receipt source commit is missing",
        )? != source
        || text(
            child.get("source_tree"),
            "lease receipt source tree is missing",
        )? != text(
            record.pointer("/handoff/tree"),
            "lease handoff tree is missing",
        )?
    {
        return Err(invalid(
            "lease receipt child is not bound to its source handoff",
        ));
    }
    let child_commit = text(child.get("commit"), "lease receipt child commit is missing")?;
    if git(
        reads,
        root,
        &["rev-parse", &format!("{child_commit}^{{tree}}")],
    )? != text(child.get("tree"), "lease receipt child tree is missing")?
        || git(reads, root, &["rev-parse", &format!("{child_commit}^")])?
            != text(
                child.get("parent_commit"),
                "lease receipt parent commit is missing",
            )?
        || text(
            child.get("parent_commit"),
            "lease receipt parent commit is missing",
        )? != source
        || git(reads, root, &["rev-parse", &format!("{source}^{{tree}}")])?
            != text(
                child.get("parent_tree"),
                "lease receipt parent tree is missing",
            )?
    {
        return Err(invalid(
            "lease receipt child is not adjacent to source handoff",
        ));
    }
    let child_paths = git(
        reads,
        root,
        &[
            "diff",
            "--name-only",
            "--no-renames",
            &format!("{child_commit}^"),
            child_commit,
        ],
    )?;
    if child_paths.lines().collect::<Vec<_>>() != [RECEIPT] {
        return Err(invalid(
            "lease receipt child does not contain exactly one receipt path",
        ));
    }
    let child_receipt = git(
        reads,
        root,
        &["ls-tree", "--name-only", child_commit, "--", RECEIPT],
    )?;
    if child_receipt != RECEIPT {
        return Err(invalid(
            "lease receipt child does not contain exactly one receipt path",
        ));
    }
    let source_paths = git(
        reads,
        root,
        &["diff", "--name-only", "--no-renames", base, source],
    )?;
    if source_paths.lines().any(|path| path == RECEIPT) {
        return Err(invalid("lease source handoff contains its receipt child"));
    }
    let source_receipt = git(
        reads,
        root,
        &["ls-tree", "--name-only", source, "--", RECEIPT],
    )?;
    if !source_receipt.is_empty() {
        return Err(invalid("lease source handoff contains its receipt child"));
    }
    package_excluded(reads, root, source)?;
    if record.get("ready_receipt").and_then(Value::as_str) != Some(RECEIPT) {
        return Err(invalid(
            "lease ready receipt does not name its sole receipt child",
        ));
    }
    Ok(())
}

fn package_excluded(reads: &ReadSession, root: &Path, source: &str) -> Result<(), InventoryError> {
    if !package_digest_excluded(RECEIPT) {
        return Err(invalid(
            "lease receipt child is not excluded from package authority",
        ));
    }
    let bytes = git(
        reads,
        root,
        &["show", &format!("{source}:plugin-manifest-draft.json")],
    )?;
    let manifest: Value = serde_json::from_str(&bytes)
        .map_err(|_| invalid("lease source package manifest is malformed"))?;
    if inventory_paths(&manifest)
        .iter()
        .any(|path| path == RECEIPT)
    {
        return Err(invalid("lease receipt child is package-visible"));
    }
    Ok(())
}

fn handoff(record: &Value) -> Result<&str, InventoryError> {
    if record.pointer("/handoff/status").and_then(Value::as_str) != Some("verified") {
        return Err(invalid("lease handoff is not verified"));
    }
    text(
        record.pointer("/handoff/commit"),
        "lease handoff commit is missing",
    )
}

fn git(reads: &ReadSession, root: &Path, args: &[&str]) -> Result<String, InventoryError> {
    super::git_query::text(reads, root, args, "lease handoff Git identity")
}
fn text<'a>(value: Option<&'a Value>, message: &str) -> Result<&'a str, InventoryError> {
    value
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(message))
}
fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}
