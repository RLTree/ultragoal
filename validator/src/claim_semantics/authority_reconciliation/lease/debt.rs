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
        out.push(Failure::new(
            "authority-lease",
            "p0_exception_lane_mismatch",
            "lane_id",
        ));
    }
    let diagnostic_paths = overlap::array_set(record, "diagnostic_paths");
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
    let exact_paths = !allowed_paths.is_empty()
        && diagnostic_paths == allowed_paths
        && !diagnostic_paths.is_empty()
        && diagnostic_paths.iter().all(|path| {
            !path.contains('*')
                && !Path::new(path).is_absolute()
                && !Path::new(path)
                    .components()
                    .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
                && root.join(path).is_file()
                && overlap::normalize_path(root, &authority, path).is_some()
                && owned_files.contains(path)
        })
        && owned_files
            .iter()
            .all(|path| diagnostic_paths.contains(path));
    if !exact_paths {
        out.push(Failure::new(
            "authority-lease",
            "exact_diagnostic_path_membership_missing",
            "allowed.paths/diagnostic_paths/owned_files/exact_current_set",
        ));
    }
    let forbidden_paths = registry
        .pointer("/lease_state/p0_exception/forbidden/paths")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    for path in &diagnostic_paths {
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
        out.push(Failure::new(
            "authority-lease",
            "diagnostic_path_set_digest_mismatch",
            "diagnostic_path_set_digest",
        ));
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
        (None, Some(_)) => out.push(Failure::new(
            "authority-lease",
            "diagnostic_output_digest_without_path",
            "diagnostic_output_path",
        )),
        (Some(path), digest) => {
            let safe = !Path::new(path).is_absolute()
                && !Path::new(path).components().any(|component| {
                    matches!(component, Component::ParentDir | Component::RootDir)
                })
                && root.join(path).is_file()
                && overlap::normalize_path(root, &authority, path).is_some();
            if !safe {
                out.push(Failure::new(
                    "authority-lease",
                    "diagnostic_output_path_unsafe",
                    path,
                ));
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
    for key in [
        "diagnostic_source_kind",
        "operation_id",
        "tool",
        "observed_at",
    ] {
        if record
            .get(key)
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            out.push(Failure::new(
                "authority-lease",
                "exact_diagnostic_binding_missing",
                key,
            ));
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
        out.push(Failure::new(
            "authority-lease",
            "p0_operation_not_allowed",
            operation,
        ));
    }
    let source_key = if operation == "retention-aware cleanup" {
        "retention"
    } else {
        operation
    };
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
        out.push(Failure::new(
            "authority-lease",
            "p0_command_source_mismatch",
            operation,
        ));
    }
    if expected_kind != record.get("diagnostic_source_kind").and_then(Value::as_str) {
        out.push(Failure::new(
            "authority-lease",
            "p0_source_kind_mismatch",
            operation,
        ));
    }
    let forbidden_operations = registry
        .pointer("/lease_state/p0_exception/forbidden/operations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str);
    for forbidden in forbidden_operations {
        if operation == forbidden {
            out.push(Failure::new(
                "authority-lease",
                "p0_forbidden_operation",
                forbidden,
            ));
        }
    }
}
