use super::super::contract_codec::{
    BoundaryLineage, ClaimCeiling, CoverageManifest, CoverageReceipt, Digest, EquivalenceStatus,
    StrictBoundaryAuthorityStatus, ToolIdentity, lineage_digest,
};
use super::super::source_digest_adapter::{self, CoverageSourceDigestRequest};
use std::path::Path;

pub(super) fn lineage_failures(
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
        "coverage_routine_manifest_digest_mismatch",
        out,
    );
    compare_file_digest(
        &super::super::coverage_command_path(root),
        receipt
            .coverage_command_digest
            .as_ref()
            .map(ToString::to_string),
        "coverage_routine_command_digest_mismatch",
        out,
    );
    compare_result(
        source_digest_adapter::execute(CoverageSourceDigestRequest::SourceTree { root, manifest })
            .map(|response| response.digest.to_string()),
        receipt.source_tree_digest.as_ref().map(ToString::to_string),
        "coverage_routine_source_tree_digest_mismatch",
        out,
    );
    let mut actual_targets = receipt.target_paths();
    let mut expected_targets = manifest.target_paths();
    actual_targets.sort();
    expected_targets.sort();
    if actual_targets != expected_targets {
        out.push("coverage_routine_manifest_target_gap".to_string());
    }
    let mut actual_dimensions = receipt.measured_dimensions.clone().unwrap_or_default();
    actual_dimensions.sort();
    actual_dimensions.dedup();
    if actual_dimensions != manifest.measured_dimensions() {
        out.push("coverage_routine_required_dimension_missing".to_string());
    }
    validate_lineage(receipt, out);
}

fn validate_lineage(receipt: &CoverageReceipt, out: &mut Vec<String>) {
    let Some(observed) = receipt.boundary_lineage_digest.as_ref() else {
        out.push("coverage_routine_boundary_lineage_missing".to_string());
        return;
    };
    let lineage = BoundaryLineage {
        mode: ClaimCeiling::RoutineRepairOnly,
        strict_boundary_mode: ToolIdentity::new("full_clean_exact_100_uncovered_records_empty"),
        source_tree_digest: receipt
            .source_tree_digest
            .clone()
            .unwrap_or_else(|| Digest::new("")),
        coverage_manifest_digest: receipt
            .coverage_manifest_digest
            .clone()
            .unwrap_or_else(|| Digest::new("")),
        coverage_command_digest: receipt
            .coverage_command_digest
            .clone()
            .unwrap_or_else(|| Digest::new("")),
    };
    if lineage_digest(&lineage)
        .map(|digest| digest != *observed)
        .unwrap_or(true)
    {
        out.push("coverage_routine_boundary_lineage_digest_mismatch".to_string());
    }
    match receipt.equivalence_status {
        Some(EquivalenceStatus::VerifiedCurrentInputEquivalent) => {
            if receipt.strict_boundary_authority_status
                != Some(StrictBoundaryAuthorityStatus::StrictBoundaryCurrentInputEquivalent)
                || receipt.strict_boundary_receipt_path.is_none()
                || receipt.strict_boundary_receipt_digest.is_none()
            {
                out.push("coverage_routine_strict_boundary_authority_missing".to_string());
            }
        }
        Some(EquivalenceStatus::RoutineFeedbackOnlyNoStrictBoundarySubstitution) => {}
        Some(EquivalenceStatus::Unknown) | None => {
            out.push("coverage_routine_equivalence_unverified".to_string())
        }
    }
    if receipt.coverage.is_none() {
        out.push("coverage_routine_percent_missing".to_string());
    }
    if receipt.uncovered_records.is_none() {
        out.push("coverage_routine_uncovered_records_missing".to_string());
    }
}

fn compare_file_digest(path: &Path, expected: Option<String>, code: &str, out: &mut Vec<String>) {
    compare_result(crate::digest::file(path), expected, code, out);
}

fn compare_result<E: std::fmt::Display>(
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
