use crate::context::ReadSession;
use crate::inventory::types::InventoryError;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub(super) fn validate(
    reads: &ReadSession,
    root: &Path,
    registry: &Value,
    record: &Value,
    base: (&str, &str),
) -> Result<(), InventoryError> {
    let declared = text(record, "worktree", "active lease worktree is missing")?;
    let branch = text(record, "branch", "active lease branch is missing")?;
    let worktree = fs::canonicalize(declared)
        .map_err(|_| invalid("active lease worktree cannot be resolved"))?;
    let worktree_root = registry
        .pointer("/lease_state/worktree_root")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("active lease worktree root is missing"))?;
    let worktree_root = fs::canonicalize(worktree_root)
        .map_err(|_| invalid("active lease worktree root cannot be resolved"))?;
    if !worktree.starts_with(&worktree_root) {
        return Err(invalid("active lease worktree is outside its root"));
    }
    let inventory = git(reads, root, &["worktree", "list", "--porcelain"])?;
    let entry = inventory
        .split("\n\n")
        .find(|entry| {
            entry
                .lines()
                .find_map(|line| line.strip_prefix("worktree "))
                .and_then(|path| fs::canonicalize(path).ok())
                .as_ref()
                == Some(&worktree)
        })
        .ok_or_else(|| invalid("active lease worktree is not registered"))?;
    if field(entry, "HEAD ") != Some(base.0)
        || field(entry, "branch ") != Some(&format!("refs/heads/{branch}"))
    {
        return Err(invalid(
            "active lease worktree identity differs from its lease",
        ));
    }
    if git(reads, &worktree, &["rev-parse", "HEAD^{tree}"])? != base.1 {
        return Err(invalid("active lease worktree tree differs from its lease"));
    }
    if !git(
        reads,
        &worktree,
        &["status", "--porcelain", "--untracked-files=all"],
    )?
    .is_empty()
    {
        return Err(invalid("active lease worktree is dirty"));
    }
    Ok(())
}

fn field<'a>(entry: &'a str, prefix: &str) -> Option<&'a str> {
    entry.lines().find_map(|line| line.strip_prefix(prefix))
}

fn git(reads: &ReadSession, root: &Path, arguments: &[&str]) -> Result<String, InventoryError> {
    super::super::git_query::text(reads, root, arguments, "active lease worktree identity")
}

fn text<'a>(record: &'a Value, field: &str, message: &str) -> Result<&'a str, InventoryError> {
    record
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(message))
}

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}
