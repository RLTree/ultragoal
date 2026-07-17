use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

pub(super) fn check(registry: &Value, root: &Path, out: &mut Vec<Failure>) {
    let Some(checkpoint_commit) = registry
        .pointer("/root_freeze/protected_base/commit")
        .and_then(Value::as_str)
    else {
        return;
    };
    let checkpoint_tree = registry
        .pointer("/root_freeze/protected_base/tree")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let current_commit = git(root, &["rev-parse", "HEAD"]);
    let current_tree = git(root, &["rev-parse", "HEAD^{tree}"]);
    if registry
        .pointer("/root_freeze/expected_branch")
        .and_then(Value::as_str)
        .unwrap_or("")
        != git(root, &["branch", "--show-current"])
    {
        out.push(Failure::new(
            "authority-freeze",
            "protected_branch_mismatch",
            "expected_branch",
        ));
    }
    let changed = git_lines(
        root,
        &["diff", "--name-only", &format!("{checkpoint_commit}..HEAD")],
    );
    let permitted = registry
        .pointer("/root_freeze/permitted_root_paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let active_p0 = registry
        .pointer("/lease_state/active_records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|record| str_field(record, "exception_id") == "P0-DEBT-REPAIR");
    let p0_allowed = registry
        .pointer("/lease_state/p0_exception/allowed/paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let clean = git(root, &["status", "--porcelain"]).is_empty();
    if !clean || !is_ancestor(root, checkpoint_commit, &current_commit) {
        out.push(Failure::new(
            "authority-freeze",
            "protected_base_mismatch",
            format!("{checkpoint_commit}/{checkpoint_tree}"),
        ));
    }
    if git(
        root,
        &["rev-parse", &format!("{checkpoint_commit}^{{tree}}")],
    ) != checkpoint_tree
    {
        out.push(Failure::new(
            "authority-freeze",
            "protected_base_tree_mismatch",
            checkpoint_commit,
        ));
    }
    if changed.iter().any(|path| {
        !permitted.contains(path.as_str())
            && (!active_p0 || !p0_allowed.iter().any(|pattern| path_matches(pattern, path)))
    }) {
        out.push(Failure::new(
            "authority-freeze",
            "unexpected_changed_path",
            changed.join(","),
        ));
    }
    if active_p0 {
        check_p0_transition_and_scope(registry, root, &current_commit, out);
    }
    if changed.iter().any(|path| {
        path == ".git"
            || path.starts_with(".git/")
            || path == ".codex/agents"
            || path.starts_with(".codex/agents/")
    }) {
        out.push(Failure::new(
            "authority-freeze",
            "protected_root_changed",
            changed.join(","),
        ));
    }
    if current_commit.is_empty() || current_tree.is_empty() {
        out.push(Failure::new(
            "authority-freeze",
            "candidate_identity_unavailable",
            "HEAD",
        ));
    }
    if str_field(registry, "authority_mode") != "single_root_writer_single_successor_registry" {
        out.push(Failure::new(
            "authority-freeze",
            "root_authority_mode_mismatch",
            "authority_mode",
        ));
    }
}

fn is_ancestor(root: &Path, ancestor: &str, current: &str) -> bool {
    !current.is_empty()
        && Command::new("git")
            .args(["merge-base", "--is-ancestor", ancestor, current])
            .current_dir(root)
            .status()
            .is_ok_and(|status| status.success())
}

fn git(root: &Path, args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

fn git_lines(root: &Path, args: &[&str]) -> Vec<String> {
    git(root, args)
        .lines()
        .map(str::to_owned)
        .filter(|line| !line.is_empty())
        .collect()
}

fn path_matches(pattern: &str, path: &str) -> bool {
    pattern == path
        || pattern
            .strip_suffix("/**")
            .is_some_and(|prefix| path == prefix || path.starts_with(&format!("{prefix}/")))
}

fn check_p0_transition_and_scope(
    registry: &Value,
    root: &Path,
    candidate: &str,
    out: &mut Vec<Failure>,
) {
    let transition = registry.pointer("/lease_state/p0_exception/authority_transition");
    let issued = transition
        .and_then(|row| row.get("status"))
        .and_then(Value::as_str)
        == Some("recorded");
    let commit = transition
        .and_then(|row| row.get("commit"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let paths = transition
        .and_then(|row| row.get("paths"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let path_digest = transition
        .and_then(|row| row.get("paths"))
        .and_then(|paths| serde_json::to_vec(paths).ok())
        .map(|bytes| crate::digest::bytes(&bytes))
        .unwrap_or_default();
    let allowed = registry
        .pointer("/lease_state/p0_exception/allowed/paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let work = git_lines(
        root,
        &["diff", "--name-only", &format!("{commit}..{candidate}")],
    );
    let activation_paths = work
        .iter()
        .filter(|path| !allowed.iter().any(|pattern| path_matches(pattern, path)))
        .cloned()
        .collect::<BTreeSet<_>>();
    let observed_tree = git(root, &["rev-parse", &format!("{commit}^{{tree}}")]);
    let parent = git(root, &["rev-parse", &format!("{commit}^")]);
    let parent_tree = git(root, &["rev-parse", &format!("{parent}^{{tree}}")]);
    let transition_ok = issued
        && !commit.is_empty()
        && is_ancestor(root, commit, candidate)
        && transition
            .and_then(|row| row.get("tree"))
            .and_then(Value::as_str)
            == Some(observed_tree.as_str())
        && transition
            .and_then(|row| row.get("parent_commit"))
            .and_then(Value::as_str)
            == Some(parent.as_str())
        && transition
            .and_then(|row| row.get("parent_tree"))
            .and_then(Value::as_str)
            == Some(parent_tree.as_str())
        && transition
            .and_then(|row| row.get("path_set_digest"))
            .and_then(Value::as_str)
            == Some(path_digest.as_str())
        && !paths.is_empty()
        && paths == activation_paths;
    if !transition_ok {
        out.push(Failure::new(
            "authority-freeze",
            "p0_authority_transition_invalid",
            "authority_transition",
        ));
        return;
    }
}
