use super::CoverageExecution;
use super::contract_codec::{
    BlockedClaimClass, BoundaryLineage, ClaimCeiling, ClaimId, CoverageCacheClass,
    CoverageCommandLine, CoverageMeasurement, CoveragePolicy, CoverageReceipt, CoverageTargetDir,
    Digest, EquivalenceStatus, MachineReadableReport, Reason, RepositoryPath, SchemaId,
    StrictBoundaryAuthorityStatus, SupportedClaimClass, TargetRevision, TargetRevisionKind,
    ToolIdentity, WorkspaceRoot, lineage_digest, read_manifest, write_receipt,
};
use super::coverage_tool_effect_adapter::{
    self, CoverageTool, CoverageToolEffectRequest, CoverageToolEffectResponse,
};
use super::report_codec_adapter::{
    self, CoverageReportObservation, CoverageReportRequest, CoverageReportResponse,
};
use super::source_digest_adapter::{self, CoverageSourceDigestRequest};
use std::fs;
use std::path::Path;

const ROUTINE_REPORT_REL: &str = "validation_artifacts/coverage/llvm-cov-routine.json";
const ROUTINE_TARGET_DIR: &str = "target/ultragoal-routine-coverage";

pub(super) fn execute(root: &Path, receipt: &Path, candidate: &str) -> CoverageExecution {
    match execute_inner(root, receipt, candidate) {
        Ok(()) => CoverageExecution {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
            cache_mode: "coverage_routine_verified_local",
        },
        Err(error) => CoverageExecution {
            code: 2,
            stdout: String::new(),
            stderr: format!("coverage_routine_observation_failed:{error}"),
            cache_mode: "coverage_routine_verified_local",
        },
    }
}

fn execute_inner(root: &Path, receipt: &Path, candidate: &str) -> Result<(), String> {
    let report = crate::output_path::literal_claim_artifact_path(
        root,
        ROUTINE_REPORT_REL,
        "routine coverage report",
    );
    if let Some(parent) = report.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create routine report dir:{error}"))?;
    }
    coverage_tool_effect_adapter::execute(CoverageToolEffectRequest::RunRoutine {
        root,
        report: &report,
        target_dir: ROUTINE_TARGET_DIR,
    })
    .map_err(|error| error.to_string())?;
    let observation = match report_codec_adapter::execute(CoverageReportRequest::Observe {
        root,
        path: &report,
    })
    .map_err(|error| error.to_string())?
    {
        CoverageReportResponse::Observation(observation) => observation,
        CoverageReportResponse::Validated => unreachable!("observe request returns observation"),
    };
    write_routine_receipt(root, receipt, candidate, &report, observation)
}

fn write_routine_receipt(
    root: &Path,
    receipt: &Path,
    candidate: &str,
    report: &Path,
    observation: CoverageReportObservation,
) -> Result<(), String> {
    let manifest_path = super::coverage_manifest_path(root);
    let command_path = super::coverage_command_path(root);
    let manifest = read_manifest(&manifest_path).map_err(|error| error.to_string())?;
    let source_digest = source_digest_adapter::execute(CoverageSourceDigestRequest::SourceTree {
        root,
        manifest: &manifest,
    })
    .map_err(|error| error.to_string())?
    .digest;
    let changed_digest =
        source_digest_adapter::execute(CoverageSourceDigestRequest::ChangedFiles {
            root,
            manifest: &manifest,
        })
        .map_err(|error| error.to_string())?
        .digest;
    let manifest_digest = Digest::new(crate::digest::file(&manifest_path)?);
    let command_digest = Digest::new(crate::digest::file(&command_path)?);
    let lineage = BoundaryLineage {
        mode: ClaimCeiling::RoutineRepairOnly,
        strict_boundary_mode: ToolIdentity::new("full_clean_exact_100_uncovered_records_empty"),
        source_tree_digest: source_digest.clone(),
        coverage_manifest_digest: manifest_digest.clone(),
        coverage_command_digest: command_digest.clone(),
    };
    let uncovered_records = observation
        .uncovered_files
        .into_iter()
        .map(|file| super::contract_codec::UncoveredRecord {
            path: RepositoryPath::new(file.path),
            reason: Reason::new(format!("routine line coverage {:.2}%", file.line_percent)),
            owner: None,
            blocker_or_debt_id: None,
        })
        .collect::<Vec<_>>();
    let receipt_record = CoverageReceipt {
        schema: Some(SchemaId::new("harness-ultragoal.coverage-receipt.v1")),
        claim_id: Some(ClaimId::new("CLAIM-001")),
        command: Some(CoverageCommandLine::new(
            "cargo llvm-cov --workspace --all-features --json --summary-only --offline --no-clean -- coverage",
        )),
        tool: Some(ToolIdentity::new("cargo-llvm-cov")),
        coverage_target_dir: Some(CoverageTargetDir::new(ROUTINE_TARGET_DIR)),
        source_tree_digest: Some(source_digest),
        coverage_manifest_digest: Some(manifest_digest),
        coverage_command_digest: Some(command_digest),
        changed_files_digest: Some(changed_digest),
        tool_version: Some(tool_version(CoverageTool::CargoLlvmCov)?),
        workspace_root: Some(WorkspaceRoot::new(workspace_root(root))),
        target_revision: Some(TargetRevision {
            kind: TargetRevisionKind::PackageDigest,
            value: Digest::new(candidate),
        }),
        command_started_at: None,
        command_completed_at: None,
        command_exit: Some(0),
        machine_readable_report: Some(MachineReadableReport {
            path: RepositoryPath::new(ROUTINE_REPORT_REL),
            digest: Digest::new(crate::digest::file(report)?),
        }),
        generated_by: Some(ToolIdentity::new("coverage-command")),
        percent_source: Some(ToolIdentity::new("machine_readable_report")),
        target_paths: Some(manifest.required_target_paths.clone()),
        measured_dimensions: Some(manifest.measured_dimensions()),
        coverage: Some(CoverageMeasurement {
            percent: observation.line_percent,
            floor_percent: 0.0,
            policy: CoveragePolicy::RoutineRepairFeedback,
            owner: None,
            reason: None,
            blocker_or_debt_id: None,
        }),
        uncovered_count: Some(uncovered_records.len()),
        uncovered_records: Some(uncovered_records),
        exclusions: Some(manifest.exclusions.clone()),
        generated_at: None,
        claim_ceiling: Some(ClaimCeiling::RoutineRepairOnly),
        supported_claim_classes: Some(vec![SupportedClaimClass::RoutineCoverageFeedback]),
        blocked_claim_classes: Some(routine_blocked()),
        cargo_version: Some(tool_version(CoverageTool::Cargo)?),
        rustc_version: Some(tool_version(CoverageTool::Rustc)?),
        flags: Some(routine_flags()),
        coverage_cache_class: Some(CoverageCacheClass::RetainedArtifactVerifiedLocal),
        boundary_lineage_digest: Some(lineage_digest(&lineage).map_err(|error| error.to_string())?),
        boundary_lineage: Some(lineage),
        strict_boundary_authority_status: Some(StrictBoundaryAuthorityStatus::StrictBoundaryNotRun),
        strict_boundary_required_for_claims: Some(strict_required()),
        equivalence_status: Some(
            EquivalenceStatus::RoutineFeedbackOnlyNoStrictBoundarySubstitution,
        ),
        current_candidate_digest: Some(Digest::new(candidate)),
        strict_boundary_receipt_path: None,
        strict_boundary_receipt_digest: None,
    };
    let resolved = crate::output_path::claim_artifact_path(root, receipt, "coverage receipt")?;
    write_receipt(&resolved, &receipt_record).map_err(|error| error.to_string())
}

fn tool_version(tool: CoverageTool) -> Result<super::contract_codec::ToolVersion, String> {
    match coverage_tool_effect_adapter::execute(CoverageToolEffectRequest::ReadVersion { tool })
        .map_err(|error| error.to_string())?
    {
        CoverageToolEffectResponse::Version(version) => Ok(version),
        CoverageToolEffectResponse::RoutineCompleted
        | CoverageToolEffectResponse::StrictCompleted => {
            Err("coverage_tool_version_response_mismatch".to_string())
        }
    }
}

fn workspace_root(root: &Path) -> String {
    root.canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn routine_flags() -> Vec<CoverageCommandLine> {
    [
        "--json",
        "--summary-only",
        "--offline",
        "--no-clean",
        "--",
        "coverage",
    ]
    .into_iter()
    .map(CoverageCommandLine::new)
    .collect()
}

fn routine_blocked() -> Vec<BlockedClaimClass> {
    let mut claims = strict_required();
    claims.insert(0, BlockedClaimClass::CompleteCoverage);
    claims.push(BlockedClaimClass::AppRegistryOrReviewerExposure);
    claims
}

fn strict_required() -> Vec<BlockedClaimClass> {
    use BlockedClaimClass::*;
    vec![
        Completion,
        PackageReadiness,
        ReviewReadiness,
        ReleaseReadiness,
        FinalPacketCorrectness,
        UpdateGoalEligibility,
    ]
}
