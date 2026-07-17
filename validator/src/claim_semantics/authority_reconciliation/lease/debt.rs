use super::overlap;
use crate::audit::contract::Failure;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Component, Path};

const P0_EXCEPTION: &str = "P0-DEBT-REPAIR";

pub(super) fn check(record: &Value, registry: &Value, root: &Path, out: &mut Vec<Failure>) {
    if record.get("exception_id").and_then(Value::as_str) != Some(P0_EXCEPTION) {
        return;
    }
    if record.get("lane_id").and_then(Value::as_str) != Some("P0") {
        fail(out, "p0_exception_lane_mismatch", "lane_id");
    }
    check_transition_binding(record, registry, out);
    let diagnostic_paths = overlap::array_set(record, "diagnostic_paths");
    let support_files = overlap::array_set(record, "support_files");
    let owned_files = overlap::array_set(record, "owned_files");
    let allowed_paths = registry
        .pointer("/lease_state/p0_exception/allowed/paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let authority = root.to_string_lossy();
    if !record.get("support_files").is_some_and(Value::is_array) {
        fail(out, "p0_support_files_missing", "support_files");
    }
    let diagnostics = normalized_files(root, &diagnostic_paths, out);
    let support = normalized_files(root, &support_files, out);
    let owned = normalized_files(root, &owned_files, out);
    let allowed = normalized_files(root, &allowed_paths, out);
    if let Some(path) = diagnostics.intersection(&support).next() {
        fail(out, "p0_support_file_overlaps_diagnostic", path);
    }
    let raw_union = diagnostic_paths.union(&support_files).cloned().collect();
    let authority_union = diagnostics.union(&support).cloned().collect();
    if owned_files != raw_union
        || allowed_paths != raw_union
        || owned != authority_union
        || allowed != authority_union
    {
        fail(
            out,
            "p0_dependency_closed_path_union_mismatch",
            "allowed.paths/diagnostic_paths/support_files/owned_files",
        );
    }
    let forbidden_paths = registry
        .pointer("/lease_state/p0_exception/forbidden/paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    for path in diagnostic_paths.union(&support_files) {
        if forbidden_paths.iter().any(|pattern| {
            let raw_match = pattern
                .strip_suffix("/**")
                .map_or(*pattern == path, |prefix| {
                    path == prefix || path.starts_with(&format!("{prefix}/"))
                });
            let normalized_match = overlap::normalize_path(root, &authority, path)
                .zip(pattern.strip_suffix("/**").or(Some(pattern)))
                .and_then(|(normalized, prefix)| {
                    overlap::normalize_path(root, &authority, prefix).map(|forbidden| {
                        normalized == forbidden || normalized.starts_with(&format!("{forbidden}/"))
                    })
                })
                .unwrap_or(false);
            raw_match || normalized_match
        }) {
            out.push(Failure::new("authority-lease", "p0_forbidden_path", path));
        }
    }
    let expected_path_digest = serde_json::to_vec(&diagnostic_paths)
        .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
        .unwrap_or_default();
    if record
        .get("diagnostic_path_set_digest")
        .and_then(Value::as_str)
        != Some(expected_path_digest.as_str())
    {
        fail(
            out,
            "diagnostic_path_set_digest_mismatch",
            "diagnostic_path_set_digest",
        );
    }
    let output_path = record
        .get("diagnostic_output_path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty());
    let output_digest = record
        .get("diagnostic_output_digest")
        .and_then(Value::as_str)
        .filter(|digest| !digest.is_empty());
    match (output_path, output_digest) {
        (None, Some(_)) => fail(
            out,
            "diagnostic_output_digest_without_path",
            "diagnostic_output_path",
        ),
        (Some(path), digest) => {
            let safe = !Path::new(path).is_absolute()
                && !Path::new(path).components().any(|component| {
                    matches!(component, Component::ParentDir | Component::RootDir)
                })
                && root.join(path).is_file()
                && overlap::normalize_path(root, &authority, path).is_some();
            if !safe {
                fail(out, "diagnostic_output_path_unsafe", path);
            } else if let Some(expected) = digest
                && crate::digest::file(&root.join(path)).ok().as_deref() != Some(expected)
            {
                out.push(Failure::new(
                    "authority-lease",
                    "diagnostic_output_digest_mismatch",
                    path,
                ));
            }
        }
        (None, None) => {}
    }
    for key in "diagnostic_source_kind operation_id tool observed_at".split(' ') {
        if record
            .get(key)
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            fail(out, "exact_diagnostic_binding_missing", key);
        }
    }
    let operation = record
        .get("operation_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let allowed_operations = registry
        .pointer("/lease_state/p0_exception/allowed/operations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if !allowed_operations.contains(operation) {
        fail(out, "p0_operation_not_allowed", operation);
    }
    let source_key = (operation == "retention-aware cleanup")
        .then_some("retention")
        .unwrap_or(operation);
    let source = registry.pointer(&format!(
        "/lease_state/p0_exception/debt_path_sources/{source_key}"
    ));
    let expected_command = source
        .and_then(|row| row.get("command"))
        .and_then(Value::as_str);
    let expected_kind = source
        .and_then(|row| row.get("source_kind"))
        .and_then(Value::as_str);
    let commands = record
        .get("commands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    if expected_command.is_none_or(|command| !commands.contains(&command)) {
        fail(out, "p0_command_source_mismatch", operation);
    }
    if expected_kind != record.get("diagnostic_source_kind").and_then(Value::as_str) {
        fail(out, "p0_source_kind_mismatch", operation);
    }
    let forbidden_operations = registry
        .pointer("/lease_state/p0_exception/forbidden/operations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str);
    for forbidden in forbidden_operations {
        if operation == forbidden {
            fail(out, "p0_forbidden_operation", forbidden);
        }
    }
}

fn fail(out: &mut Vec<Failure>, error: &str, detail: impl Into<String>) {
    out.push(Failure::new("authority-lease", error, detail));
}

fn normalized_files(
    root: &Path,
    paths: &BTreeSet<String>,
    out: &mut Vec<Failure>,
) -> BTreeSet<String> {
    let mut normalized = BTreeSet::new();
    let canonical_root = root.canonicalize().ok();
    for raw in paths {
        let safe = !raw.contains('*')
            && !Path::new(raw).is_absolute()
            && Path::new(raw)
                .components()
                .all(|part| !matches!(part, Component::ParentDir | Component::RootDir))
            && root.join(raw).is_file();
        let Some(path) = safe
            .then(|| root.join(raw).canonicalize().ok())
            .flatten()
            .filter(|path| {
                canonical_root
                    .as_ref()
                    .is_some_and(|base| path.starts_with(base))
            })
        else {
            fail(out, "p0_exact_file_invalid", raw);
            continue;
        };
        if !normalized.insert(path.to_string_lossy().into_owned()) {
            fail(out, "p0_normalized_file_collision", raw);
        }
    }
    normalized
}

fn check_transition_binding(record: &Value, registry: &Value, out: &mut Vec<Failure>) {
    let transition = registry.pointer("/lease_state/p0_exception/authority_transition");
    let commit = transition
        .and_then(|row| row.get("commit"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let tree = transition
        .and_then(|row| row.get("tree"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if transition
        .and_then(|row| row.get("status"))
        .and_then(Value::as_str)
        != Some("recorded")
        || record.get("base_commit").and_then(Value::as_str) != Some(commit)
        || record.get("base_tree").and_then(Value::as_str) != Some(tree)
    {
        fail(
            out,
            "p0_transition_binding_mismatch",
            "authority_transition/base_commit/base_tree",
        );
    }
}
