use crate::inventory::types::InventoryError;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

pub(super) fn validate(
    root: &Path,
    registry: &Value,
    record: &Value,
    base: (&str, &str),
) -> Result<(), InventoryError> {
    let declared = text(record, "worktree", "P0 worktree is missing")?;
    let branch = text(record, "branch", "P0 branch is missing")?;
    let worktree = fs::canonicalize(declared)
        .map_err(|_| invalid("P0 worktree does not exist or cannot be resolved"))?;
    let worktree_root = registry
        .pointer("/lease_state/worktree_root")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("P0 worktree root is missing"))?;
    let worktree_root = fs::canonicalize(worktree_root)
        .map_err(|_| invalid("P0 worktree root does not exist or cannot be resolved"))?;
    if !worktree.starts_with(&worktree_root) {
        return Err(invalid(
            "P0 worktree is outside the configured worktree root",
        ));
    }
    let inventory = git(root, &["worktree", "list", "--porcelain"])?;
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
        .ok_or_else(|| invalid("P0 worktree is not registered"))?;
    if field(entry, "HEAD ") != Some(base.0)
        || field(entry, "branch ") != Some(&format!("refs/heads/{branch}"))
    {
        return Err(invalid("P0 worktree head or branch differs from its lease"));
    }
    if git(&worktree, &["rev-parse", "HEAD^{tree}"])? != base.1 {
        return Err(invalid("P0 worktree tree differs from its lease"));
    }
    if !git(
        &worktree,
        &["status", "--porcelain", "--untracked-files=all"],
    )?
    .is_empty()
    {
        return Err(invalid("P0 worktree is dirty"));
    }
    Ok(())
}

fn field<'a>(entry: &'a str, prefix: &str) -> Option<&'a str> {
    entry.lines().find_map(|line| line.strip_prefix(prefix))
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, InventoryError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|_| invalid("cannot inspect P0 worktree identity"))?;
    if !output.status.success() {
        return Err(invalid("cannot inspect P0 worktree identity"));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| invalid("P0 worktree identity is not UTF-8"))
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
