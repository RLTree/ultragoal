use super::super::contract_codec::{
    BlockedClaimClass, ClaimCeiling, CoverageCacheClass, CoverageReceipt, SupportedClaimClass,
    TargetRevisionKind,
};
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
            "coverage_routine_receipt_malformed",
        ),
        (
            CoverageReceipt::text(&receipt.command).is_empty(),
            "coverage_routine_command_missing",
        ),
        (
            CoverageReceipt::text(&receipt.tool).is_empty(),
            "coverage_routine_tool_missing",
        ),
        (
            CoverageReceipt::text(&receipt.tool_version).is_empty(),
            "coverage_routine_tool_version_missing",
        ),
        (
            CoverageReceipt::text(&receipt.cargo_version).is_empty(),
            "coverage_routine_cargo_version_missing",
        ),
        (
            CoverageReceipt::text(&receipt.rustc_version).is_empty(),
            "coverage_routine_rustc_version_missing",
        ),
        (
            receipt.coverage_cache_class.is_none(),
            "coverage_routine_cache_class_missing",
        ),
        (
            receipt.command_exit != Some(0),
            "coverage_routine_command_failed",
        ),
    ] {
        if bad {
            out.push(code.to_string());
        }
    }
    if receipt.coverage_cache_class.is_some()
        && receipt.coverage_cache_class != Some(CoverageCacheClass::RetainedArtifactVerifiedLocal)
    {
        out.push("coverage_routine_cache_class_not_verified_local".to_string());
    }
    match target_dir::status(root, CoverageReceipt::text(&receipt.coverage_target_dir)) {
        TargetDirStatus::Missing => out.push("coverage_routine_target_dir_missing".to_string()),
        TargetDirStatus::NotIsolated => {
            out.push("coverage_routine_target_dir_not_isolated".to_string())
        }
        TargetDirStatus::Isolated => {}
    }
    let target = receipt.target_revision.as_ref();
    if target.map(|target| target.kind) != Some(TargetRevisionKind::PackageDigest) {
        out.push("coverage_routine_target_kind_mismatch".to_string());
    }
    if target.map(|target| target.value.as_str()) != Some(candidate) {
        out.push("coverage_routine_current_candidate_digest_mismatch".to_string());
    }
    if receipt.target_paths.as_ref().is_none_or(Vec::is_empty) {
        out.push("coverage_routine_missing_target_paths".to_string());
    }
    if receipt
        .measured_dimensions
        .as_ref()
        .is_none_or(Vec::is_empty)
    {
        out.push("coverage_routine_missing_dimensions".to_string());
    }
}

pub(super) fn claim_failures(receipt: &CoverageReceipt, out: &mut Vec<String>) {
    if receipt.claim_ceiling != Some(ClaimCeiling::RoutineRepairOnly) {
        out.push("coverage_routine_claim_ceiling_not_routine_only".to_string());
    }
    if receipt
        .supported_claim_classes
        .as_deref()
        .is_some_and(|claims| claims.contains(&SupportedClaimClass::CompleteCoverage))
    {
        out.push("coverage_routine_supports_complete_coverage".to_string());
    }
    for claim in required_blocked() {
        if !receipt
            .blocked_claim_classes
            .as_deref()
            .is_some_and(|claims| claims.contains(&claim))
        {
            out.push(format!(
                "coverage_routine_blocked_claim_missing:{}",
                label(claim)
            ));
        }
    }
}

fn required_blocked() -> [BlockedClaimClass; 8] {
    use BlockedClaimClass::*;
    [
        CompleteCoverage,
        Completion,
        PackageReadiness,
        ReviewReadiness,
        ReleaseReadiness,
        FinalPacketCorrectness,
        UpdateGoalEligibility,
        AppRegistryOrReviewerExposure,
    ]
}

fn label(claim: BlockedClaimClass) -> &'static str {
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
