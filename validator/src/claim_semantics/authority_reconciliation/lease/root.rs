use super::overlap;
use crate::audit::contract::Failure;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn validate(
    record: &Value,
    registry: &Value,
    root: &Path,
    authority_root: &str,
    p0: bool,
    out: &mut Vec<Failure>,
) -> String {
    let authority_trusted = authority_root == "/Users/terrynoblin/.codex/worktrees"
        && Path::new(authority_root).is_absolute()
        && Path::new(authority_root)
            .canonicalize()
            .ok()
            .is_some_and(|path| {
                path.is_dir()
                    && path.file_name().and_then(|name| name.to_str()) == Some("worktrees")
                    && path
                        .parent()
                        .and_then(Path::file_name)
                        .and_then(|name| name.to_str())
                        == Some(".codex")
            });
    if !p0 && !authority_trusted {
        out.push(Failure::new(
            "authority-lease",
            "lease_worktree_root_untrusted",
            authority_root,
        ));
    }
    let root_branch = registry
        .pointer("/root/branch")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let branch = record
        .get("branch")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if (p0 && branch != root_branch) || (!p0 && !branch.starts_with("codex/")) {
        out.push(Failure::new(
            "authority-lease",
            "lease_branch_not_isolated",
            "branch",
        ));
    }
    let shared_root = registry
        .pointer("/root/workspace")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let worktree = record
        .get("worktree")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let authority = Path::new(authority_root).canonicalize().ok();
    let worktree_ok = Path::new(worktree).is_dir()
        && Path::new(worktree)
            .canonicalize()
            .ok()
            .zip(authority.as_ref())
            .is_some_and(|(path, authority)| path.starts_with(authority) && path != *authority)
        && overlap::normalize_path(root, authority_root, worktree)
            .is_some_and(|normalized| normalized != authority_root);
    let shared_bound = record.get("shared_root").and_then(Value::as_str) == Some(shared_root)
        && Path::new(shared_root).canonicalize().ok() == root.canonicalize().ok();
    if p0 {
        if !shared_bound || record.get("worktree").is_some_and(|value| !value.is_null()) {
            out.push(Failure::new(
                "authority-lease",
                "p0_shared_root_binding_mismatch",
                "shared_root/worktree",
            ));
        }
    } else if !shared_bound || worktree.is_empty() || worktree == shared_root || !worktree_ok {
        out.push(Failure::new(
            "authority-lease",
            "lease_worktree_not_contained",
            "worktree",
        ));
    }
    if p0
        && record
            .get("isolated_roots")
            .and_then(Value::as_object)
            .is_none_or(|roots| roots.values().any(|value| !value.is_null()))
    {
        out.push(Failure::new(
            "authority-lease",
            "p0_isolation_must_be_null",
            "isolated_roots",
        ));
    }
    if !p0 {
        let roots = record.get("isolated_roots").and_then(Value::as_object);
        let worktree_root = Path::new(worktree).canonicalize().ok();
        let mut seen = BTreeSet::new();
        if roots.is_none_or(|roots| {
            roots.values().any(|value| {
                let Some(path) = value.as_str() else {
                    return true;
                };
                let Some(canonical) = Path::new(path).canonicalize().ok() else {
                    return true;
                };
                path.contains("<lease>")
                    || !Path::new(path).is_absolute()
                    || !Path::new(path).is_dir()
                    || worktree_root
                        .as_ref()
                        .is_none_or(|root| !canonical.starts_with(root))
                    || overlap::normalize_path(root, authority_root, path).is_none()
                    || !seen.insert(canonical)
            })
        }) {
            out.push(Failure::new(
                "authority-lease",
                "lease_isolation_root_uncontained",
                "isolated_roots",
            ));
        }
    }
    if p0 {
        shared_root.to_owned()
    } else {
        authority_root.to_owned()
    }
}
