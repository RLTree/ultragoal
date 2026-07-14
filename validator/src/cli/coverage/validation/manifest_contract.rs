use super::super::contract_codec::{CoverageManifest, CoverageReceipt};
use super::super::source_digest_adapter::{self, CoverageSourceDigestRequest};
use std::path::{Path, PathBuf};

pub(super) fn manifest_failures(root: &Path, manifest: &CoverageManifest, out: &mut Vec<String>) {
    if manifest.schema.as_str() != "harness-ultragoal.coverage-manifest.v1" {
        out.push("coverage_receipt_malformed".to_string());
    }
    if manifest.required_target_paths.is_empty() {
        out.push("coverage_claim_missing_target_paths".to_string());
    }
    if manifest.measured_dimensions().is_empty() {
        out.push("coverage_claim_missing_dimensions".to_string());
    }
    if manifest.changed_file_coupling_policy.required
        && manifest
            .changed_file_coupling_policy
            .changed_files
            .is_empty()
    {
        out.push("coverage_changed_file_missing_from_manifest".to_string());
    }
    for changed in manifest.changed_files() {
        match confined_path(root, &changed) {
            Ok(path) if path.is_file() => {}
            _ => out.push("coverage_changed_file_missing_from_manifest".to_string()),
        }
    }
    for exclusion in &manifest.exclusions {
        if exclusion.rationale.is_empty() {
            out.push("coverage_exclusion_missing_rationale".to_string());
        }
        if !exclusion.reviewed {
            out.push("coverage_exclusion_unreviewed".to_string());
        }
        if exclusion.counts_as_covered {
            out.push("coverage_exclusion_counted_as_covered".to_string());
        }
    }
    for target in manifest.target_paths() {
        match confined_path(root, &target) {
            Ok(path) if path.exists() => {}
            _ => out.push("coverage_target_path_nonexistent".to_string()),
        }
        if Path::new(&target).components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some(".codex-worktree" | "target" | "node_modules" | "__pycache__")
            )
        }) {
            out.push("coverage_target_path_ignored_local_state".to_string());
        }
    }
}

pub(super) fn digest_failures(
    root: &Path,
    manifest: &CoverageManifest,
    receipt: &CoverageReceipt,
    out: &mut Vec<String>,
) {
    compare_file_digest(
        &super::super::coverage_manifest_path(root),
        receipt
            .coverage_manifest_digest
            .as_ref()
            .map(ToString::to_string),
        "coverage_receipt_manifest_digest_mismatch",
        out,
    );
    compare_file_digest(
        &super::super::coverage_command_path(root),
        receipt
            .coverage_command_digest
            .as_ref()
            .map(ToString::to_string),
        "coverage_receipt_command_digest_mismatch",
        out,
    );
    compare_source_digest(
        source_digest_adapter::execute(CoverageSourceDigestRequest::SourceTree { root, manifest })
            .map(|response| response.digest.to_string()),
        receipt.source_tree_digest.as_ref().map(ToString::to_string),
        "coverage_receipt_source_digest_mismatch",
        out,
    );
    compare_source_digest(
        source_digest_adapter::execute(CoverageSourceDigestRequest::ChangedFiles {
            root,
            manifest,
        })
        .map(|response| response.digest.to_string()),
        receipt
            .changed_files_digest
            .as_ref()
            .map(ToString::to_string),
        "coverage_receipt_changed_files_digest_mismatch",
        out,
    );
    let mut actual_targets = receipt.target_paths();
    let mut expected_targets = manifest.target_paths();
    actual_targets.sort();
    expected_targets.sort();
    if actual_targets != expected_targets {
        out.push("coverage_manifest_target_gap".to_string());
    }
    let mut actual_dimensions = receipt.measured_dimensions.clone().unwrap_or_default();
    actual_dimensions.sort();
    actual_dimensions.dedup();
    if actual_dimensions != manifest.measured_dimensions() {
        out.push("coverage_required_dimension_missing".to_string());
    }
}

fn confined_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    crate::package::inventory::resolve(root, relative)
}

fn compare_file_digest(path: &Path, expected: Option<String>, code: &str, out: &mut Vec<String>) {
    compare_source_digest(crate::digest::file(path), expected, code, out);
}

fn compare_source_digest<E: std::fmt::Display>(
    result: Result<String, E>,
    expected: Option<String>,
    code: &str,
    out: &mut Vec<String>,
) {
    match (result, expected) {
        (Ok(actual), Some(expected)) if actual == expected => {}
        (Ok(_), _) => out.push(code.to_string()),
        (Err(error), _) => out.push(format!("{code}:{error}")),
    }
}
