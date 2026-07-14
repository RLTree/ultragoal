use super::super::contract_codec::{
    BlockedClaimClass, ClaimCeiling, CoveragePolicy, CoverageReceipt, SupportedClaimClass,
    TargetRevisionKind,
};
use super::super::report_codec_adapter::{self, CoverageReportRequest};
use super::super::target_dir::{self, TargetDirStatus};
use std::path::Path;

pub(super) fn scalar_failures(
    root: &Path,
    receipt: &CoverageReceipt,
    candidate: &str,
    out: &mut Vec<String>,
) {
    for (bad, code) in [
        (
            CoverageReceipt::text(&receipt.schema) != "harness-ultragoal.coverage-receipt.v1",
            "coverage_receipt_malformed",
        ),
        (
            CoverageReceipt::text(&receipt.command).is_empty(),
            "coverage_command_missing",
        ),
        (
            CoverageReceipt::text(&receipt.tool).is_empty(),
            "coverage_tool_missing",
        ),
        (
            CoverageReceipt::text(&receipt.tool_version).is_empty(),
            "coverage_receipt_tool_version_missing",
        ),
        (
            CoverageReceipt::text(&receipt.generated_by) != "coverage-command",
            "coverage_receipt_not_tool_generated",
        ),
        (
            CoverageReceipt::text(&receipt.percent_source) != "machine_readable_report",
            "coverage_percent_from_prose",
        ),
        (receipt.command_exit != Some(0), "coverage_command_failed"),
    ] {
        if bad {
            out.push(code.to_string());
        }
    }
    match target_dir::status(root, CoverageReceipt::text(&receipt.coverage_target_dir)) {
        TargetDirStatus::Missing => out.push("coverage_target_dir_missing".to_string()),
        TargetDirStatus::NotIsolated => out.push("coverage_target_dir_not_isolated".to_string()),
        TargetDirStatus::Isolated => {}
    }
    let root_string = root
        .canonicalize()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    let workspace = CoverageReceipt::text(&receipt.workspace_root);
    if workspace != "/repo" && workspace != root_string {
        out.push("coverage_receipt_workspace_mismatch".to_string());
    }
    let target = receipt.target_revision.as_ref();
    if target.map(|target| target.kind) != Some(TargetRevisionKind::PackageDigest) {
        out.push("coverage_receipt_target_kind_mismatch".to_string());
    }
    if target.map(|target| target.value.as_str()) != Some(candidate) {
        out.push(format!(
            "coverage_receipt_target_digest_mismatch:expected={candidate} actual={}",
            target.map(|target| target.value.as_str()).unwrap_or("")
        ));
    }
    if receipt.target_paths.as_ref().is_none_or(Vec::is_empty) {
        out.push("coverage_claim_missing_target_paths".to_string());
    }
    if receipt
        .measured_dimensions
        .as_ref()
        .is_none_or(Vec::is_empty)
    {
        out.push("coverage_claim_missing_dimensions".to_string());
    }
}

pub(super) fn report_failures(root: &Path, receipt: &CoverageReceipt, out: &mut Vec<String>) {
    let Some(report) = receipt.machine_readable_report.as_ref() else {
        out.push("coverage_report_missing".to_string());
        return;
    };
    let resolved = match crate::package::inventory::resolve(root, report.path.as_str()) {
        Ok(path) => path,
        Err(error) => {
            out.push(format!("coverage_report_missing:{error}"));
            return;
        }
    };
    if let Err(error) =
        report_codec_adapter::execute(CoverageReportRequest::Validate { path: &resolved })
    {
        out.push(format!("coverage_report_not_machine_readable:{error}"));
    }
    match crate::digest::file(&resolved) {
        Ok(actual) if actual == report.digest.as_str() => {}
        Ok(_) => out.push("coverage_report_digest_mismatch".to_string()),
        Err(error) => out.push(format!("coverage_report_digest_mismatch:{error}")),
    }
}

pub(super) fn completion_failures(receipt: &CoverageReceipt, out: &mut Vec<String>) {
    let coverage = receipt.coverage.as_ref();
    if coverage.map(|coverage| coverage.policy) != Some(CoveragePolicy::HundredPercentRequired) {
        out.push("coverage_ratchet_presented_as_complete".to_string());
    }
    if coverage.map(|coverage| coverage.percent) != Some(100.0) {
        out.push("coverage_claim_uncovered_code".to_string());
    }
    let uncovered = receipt.uncovered_records.as_deref().unwrap_or_default();
    if !uncovered.is_empty() {
        out.push("coverage_claim_uncovered_code".to_string());
        let first = uncovered
            .iter()
            .take(5)
            .map(|record| format!("{} ({})", record.path, record.reason))
            .collect::<Vec<_>>()
            .join(" | ");
        out.push(format!(
            "coverage_uncovered_records:total={} first={first}",
            uncovered.len()
        ));
    }
    if receipt.claim_ceiling != Some(ClaimCeiling::SupportsCompleteCoverageClaim)
        || !contains(
            receipt.supported_claim_classes.as_deref(),
            SupportedClaimClass::CompleteCoverage,
        )
    {
        out.push("coverage_ratchet_presented_as_complete".to_string());
    }
    for claim in required_blocked() {
        if !contains(receipt.blocked_claim_classes.as_deref(), claim) {
            out.push(format!(
                "coverage_blocked_claim_missing:{}",
                blocked_label(claim)
            ));
        }
    }
}

fn blocked_label(claim: BlockedClaimClass) -> &'static str {
    use BlockedClaimClass::*;
    match claim {
        CompleteCoverage => "complete_coverage",
        Completion => "completion",
        PackageReadiness => "package_readiness",
        ReviewReadiness => "review_readiness",
        ReleaseReadiness => "release_readiness",
        FinalPacketCorrectness => "final_packet_correctness",
        UpdateGoalEligibility => "update_goal_eligibility",
        AppRegistryOrReviewerExposure => "app_registry_or_reviewer_exposure",
    }
}

fn contains<T: PartialEq + Copy>(values: Option<&[T]>, needle: T) -> bool {
    values.is_some_and(|values| values.contains(&needle))
}

fn required_blocked() -> [BlockedClaimClass; 7] {
    use BlockedClaimClass::*;
    [
        Completion,
        PackageReadiness,
        ReviewReadiness,
        ReleaseReadiness,
        FinalPacketCorrectness,
        UpdateGoalEligibility,
        AppRegistryOrReviewerExposure,
    ]
}
