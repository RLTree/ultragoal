use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

pub(super) fn check(registry: &Value, root: &Path, out: &mut Vec<Failure>) {
    let Some(expected_commit) = registry
        .pointer("/root_freeze/expected_parent_commit")
        .and_then(Value::as_str)
    else {
        return;
    };
    let Some(expected_tree) = registry
        .pointer("/root_freeze/expected_parent_tree")
        .and_then(Value::as_str)
    else {
        return;
    };
    let current_commit = git(root, &["rev-parse", "HEAD"]);
    let current_tree = git(root, &["rev-parse", "HEAD^{tree}"]);
    let parent = git(root, &["rev-parse", "HEAD^"]);
    let parent_tree = git(root, &["rev-parse", "HEAD^1^{tree}"]);
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
    let committed = git_lines(
        root,
        &["diff", "--name-only", &format!("{expected_commit}..HEAD")],
    );
    let expected = registry
        .pointer("/root_freeze/expected_diff_paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let dirty_paths = git_lines(root, &["status", "--porcelain"])
        .into_iter()
        .filter_map(|line| {
            line.get(2..)
                .map(str::trim)
                .map(|path| path.trim_end_matches('/').to_owned())
        })
        .collect::<BTreeSet<_>>();
    let dirty_outside = dirty_paths.iter().any(|path| {
        !expected
            .iter()
            .any(|expected| expected == path || expected.starts_with(&format!("{path}/")))
    });
    let dirty_expected = expected
        .iter()
        .filter(|expected| {
            dirty_paths
                .iter()
                .any(|path| *expected == path || expected.starts_with(&format!("{path}/")))
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let committed_set = committed.iter().cloned().collect::<BTreeSet<_>>();
    let dirty_precommit = current_commit == expected_commit
        && current_tree == expected_tree
        && !dirty_paths.is_empty()
        && !dirty_outside
        && dirty_expected == expected;
    let clean_postcommit = parent == expected_commit
        && parent_tree == expected_tree
        && dirty_paths.is_empty()
        && committed_set == expected;
    if !dirty_precommit && !clean_postcommit {
        out.push(Failure::new(
            "authority-freeze",
            "protected_base_mismatch",
            format!("{expected_commit}/{expected_tree}"),
        ));
    }
    let actual = if dirty_precommit {
        dirty_expected.into_iter().collect()
    } else {
        committed
    };
    if actual.iter().any(|path| !expected.contains(path.as_str())) || actual.len() != expected.len()
    {
        out.push(Failure::new(
            "authority-freeze",
            "unexpected_changed_path",
            actual.join(","),
        ));
    }
    let active_p0 = registry
        .pointer("/lease_state/active_records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|record| str_field(record, "exception_id") == "P0-DEBT-REPAIR");
    if active_p0 {
        let allowed = registry
            .pointer("/lease_state/p0_exception/allowed/paths")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        if actual
            .iter()
            .any(|path| !allowed.iter().any(|pattern| path_matches(pattern, path)))
        {
            out.push(Failure::new(
                "authority-freeze",
                "p0_path_outside_allowlist",
                actual.join(","),
            ));
        }
    }
    if actual.iter().any(|path| {
        path == ".git"
            || path.starts_with(".git/")
            || path == ".codex/agents"
            || path.starts_with(".codex/agents/")
    }) {
        out.push(Failure::new(
            "authority-freeze",
            "protected_root_changed",
            actual.join(","),
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

pub(super) fn payload_refs(registry: &Value, root: &Path, out: &mut Vec<Failure>) {
    for row in registry
        .pointer("/root_freeze/payload_refs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let path = str_field(row, "path");
        let Some(path_ref) = super::safe_repo_path(root, &path) else {
            out.push(Failure::new(
                "authority-freeze",
                "payload_ref_path_unsafe",
                path,
            ));
            continue;
        };
        if str_field(row, "digest") != crate::digest::file(&path_ref).unwrap_or_default() {
            out.push(Failure::new(
                "authority-freeze",
                "payload_ref_digest_mismatch",
                path,
            ));
        }
    }
}
