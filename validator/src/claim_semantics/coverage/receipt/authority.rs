use crate::audit::contract::Failure;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(crate) struct DigestCache {
    manifest: Option<Option<Value>>,
    source_tree_digest: Option<Result<String, String>>,
    coverage_manifest_digest: Option<Result<String, String>>,
    coverage_command_digest: Option<Result<String, String>>,
    changed_files_digest: Option<Result<String, String>>,
    report_digest_by_path: BTreeMap<String, Result<String, String>>,
    report_json_valid_by_path: BTreeMap<String, bool>,
}

#[cfg(test)]
pub(crate) fn check(receipt: &Value, root: &Path, out: &mut Vec<Failure>) {
    let mut cache = DigestCache::default();
    check_with_cache(receipt, root, out, &mut cache);
}

pub(crate) fn check_with_cache(
    receipt: &Value,
    root: &Path,
    out: &mut Vec<Failure>,
    cache: &mut DigestCache,
) {
    digest_binding_checks(receipt, root, out, cache);
    scalar_authority_checks(receipt, out);
    report_checks(receipt, root, out, cache);
}

fn scalar_authority_checks(receipt: &Value, out: &mut Vec<Failure>) {
    for (bad, code) in [
        (
            str_field(receipt, "tool_version").is_empty(),
            "coverage_receipt_tool_version_missing",
        ),
        (
            str_field(receipt, "workspace_root") != "/repo",
            "coverage_receipt_workspace_mismatch",
        ),
        (
            receipt.get("command_exit").and_then(Value::as_i64) != Some(0),
            "coverage_command_failed",
        ),
        (
            str_field(receipt, "generated_by") != "coverage-command",
            "coverage_receipt_not_tool_generated",
        ),
        (
            str_field(receipt, "percent_source") != "machine_readable_report",
            "coverage_percent_from_prose",
        ),
    ] {
        if bad {
            out.push(Failure::new(
                "coverage-proof-policy",
                code,
                str_field(receipt, "claim_id"),
            ));
        }
    }
}

fn digest_binding_checks(
    receipt: &Value,
    root: &Path,
    out: &mut Vec<Failure>,
    cache: &mut DigestCache,
) {
    let manifest_path = coverage_manifest_path(root);
    let command_path = coverage_command_path(root);
    let Some(manifest) = manifest(root, cache).cloned() else {
        return;
    };
    let actual_source_tree = cached_result(&mut cache.source_tree_digest, || {
        crate::claim_semantics::coverage::digests::source_tree_digest(root, &manifest)
    });
    let actual_manifest = cached_result(&mut cache.coverage_manifest_digest, || {
        crate::digest::file(&manifest_path)
    });
    let actual_command = cached_result(&mut cache.coverage_command_digest, || {
        crate::digest::file(&command_path)
    });
    let actual_changed = cached_result(&mut cache.changed_files_digest, || {
        crate::claim_semantics::coverage::digests::changed_files_digest(root, &manifest)
    });
    for (actual, expected, code) in [
        (
            actual_source_tree,
            str_field(receipt, "source_tree_digest"),
            "coverage_receipt_source_digest_mismatch",
        ),
        (
            actual_manifest,
            str_field(receipt, "coverage_manifest_digest"),
            "coverage_receipt_manifest_digest_mismatch",
        ),
        (
            actual_command,
            str_field(receipt, "coverage_command_digest"),
            "coverage_receipt_command_digest_mismatch",
        ),
        (
            actual_changed,
            str_field(receipt, "changed_files_digest"),
            "coverage_receipt_changed_files_digest_mismatch",
        ),
    ] {
        if actual.as_deref() != Ok(expected.as_str()) {
            out.push(Failure::new("coverage-proof-policy", code, expected));
        }
    }
}

fn report_checks(receipt: &Value, root: &Path, out: &mut Vec<Failure>, cache: &mut DigestCache) {
    let report = receipt
        .get("machine_readable_report")
        .unwrap_or(&Value::Null);
    let path = str_field(report, "path");
    let digest = str_field(report, "digest");
    let actual = cache
        .report_digest_by_path
        .entry(path.clone())
        .or_insert_with(|| {
            let resolved = crate::package::inventory::resolve(root, &path)?;
            crate::digest::file(&resolved)
        })
        .clone();
    let Ok(actual) = actual else {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_report_missing",
            path,
        ));
        return;
    };
    if actual != digest {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_report_digest_mismatch",
            path.clone(),
        ));
    }
    let report_json_valid = *cache
        .report_json_valid_by_path
        .entry(path.clone())
        .or_insert_with(|| {
            crate::package::inventory::resolve(root, &path)
                .ok()
                .is_some_and(|resolved| crate::json_boundary::read_json(&resolved).is_ok())
        });
    if !report_json_valid {
        out.push(Failure::new(
            "coverage-proof-policy",
            "coverage_report_not_machine_readable",
            path,
        ));
    }
}

fn manifest<'a>(root: &Path, cache: &'a mut DigestCache) -> Option<&'a Value> {
    if cache.manifest.is_none() {
        cache.manifest = Some(crate::json_boundary::read_json(&coverage_manifest_path(root)).ok());
    }
    match &cache.manifest {
        Some(Some(value)) => Some(value),
        _ => None,
    }
}

fn cached_result(
    slot: &mut Option<Result<String, String>>,
    compute: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    if let Some(result) = slot {
        return result.clone();
    }
    let result = compute();
    *slot = Some(result.clone());
    result
}

fn coverage_manifest_path(root: &Path) -> PathBuf {
    let target = root.join(".harness/coverage-manifest.json");
    if target.is_file() {
        target
    } else {
        root.join("templates/.harness/coverage-manifest.json")
    }
}

fn coverage_command_path(root: &Path) -> PathBuf {
    let target = root.join(".harness/coverage-command");
    if target.is_file() {
        target
    } else {
        root.join("templates/.harness/coverage-command")
    }
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
#[path = "authority/tests.rs"]
mod tests;
