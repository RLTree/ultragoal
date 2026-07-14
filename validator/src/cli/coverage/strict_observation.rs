use super::CoverageExecution;
use super::contract_codec::{
    BlockedClaimClass, ClaimCeiling, ClaimId, CoverageCommandLine, CoverageMeasurement,
    CoveragePolicy, CoverageReceipt, CoverageTargetDir, Digest, MachineReadableReport, Reason,
    RepositoryPath, SchemaId, SupportedClaimClass, TargetRevision, TargetRevisionKind,
    ToolIdentity, WorkspaceRoot, read_manifest, write_receipt,
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

const STRICT_REPORT_REL: &str = "validation_artifacts/coverage/llvm-cov-full.json";
const MISSING_REPORT_REL: &str = "validation_artifacts/coverage/missing-lines.txt";
const STRICT_TARGET_DIR: &str = "target/ultragoal-coverage";

pub(super) fn execute(root: &Path, receipt: &Path, candidate: &str) -> CoverageExecution {
    match execute_inner(root, receipt, candidate) {
        Ok(()) => CoverageExecution {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
            cache_mode: "coverage_strict_no_cache",
        },
        Err(error) => CoverageExecution {
            code: 2,
            stdout: String::new(),
            stderr: format!("coverage_strict_observation_failed:{error}"),
            cache_mode: "coverage_strict_no_cache",
        },
    }
}

fn execute_inner(root: &Path, receipt: &Path, candidate: &str) -> Result<(), String> {
    let report = claim_path(root, STRICT_REPORT_REL, "strict coverage report")?;
    let missing = claim_path(root, MISSING_REPORT_REL, "strict missing-lines report")?;
    if let Some(parent) = report.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create strict report dir:{error}"))?;
    }
    coverage_tool_effect_adapter::execute(CoverageToolEffectRequest::RunStrict {
        root,
        report: &report,
        missing_report: &missing,
        target_dir: STRICT_TARGET_DIR,
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
    write_strict_receipt(root, receipt, candidate, &report, observation)
}

fn write_strict_receipt(
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
    let uncovered = observation
        .uncovered_files
        .into_iter()
        .map(|file| super::contract_codec::UncoveredRecord {
            path: RepositoryPath::new(file.path),
            reason: Reason::new(format!("line coverage {:.2}%", file.line_percent)),
            owner: None,
            blocker_or_debt_id: None,
        })
        .collect::<Vec<_>>();
    let complete = observation.line_percent == 100.0 && uncovered.is_empty();
    let record = CoverageReceipt {
        schema: Some(SchemaId::new("harness-ultragoal.coverage-receipt.v1")),
        claim_id: Some(ClaimId::new("CLAIM-001")),
        command: Some(CoverageCommandLine::new(
            "cargo llvm-cov --workspace --all-features --json --offline",
        )),
        tool: Some(ToolIdentity::new("cargo-llvm-cov")),
        coverage_target_dir: Some(CoverageTargetDir::new(STRICT_TARGET_DIR)),
        source_tree_digest: Some(source_digest),
        coverage_manifest_digest: Some(Digest::new(crate::digest::file(&manifest_path)?)),
        coverage_command_digest: Some(Digest::new(crate::digest::file(&command_path)?)),
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
            path: RepositoryPath::new(STRICT_REPORT_REL),
            digest: Digest::new(crate::digest::file(report)?),
        }),
        generated_by: Some(ToolIdentity::new("coverage-command")),
        percent_source: Some(ToolIdentity::new("machine_readable_report")),
        target_paths: Some(manifest.required_target_paths.clone()),
        measured_dimensions: Some(manifest.measured_dimensions()),
        coverage: Some(CoverageMeasurement {
            percent: observation.line_percent,
            floor_percent: 100.0,
            policy: CoveragePolicy::HundredPercentRequired,
            owner: None,
            reason: None,
            blocker_or_debt_id: None,
        }),
        uncovered_count: Some(uncovered.len()),
        uncovered_records: Some(uncovered),
        exclusions: Some(manifest.exclusions.clone()),
        generated_at: None,
        claim_ceiling: Some(if complete {
            ClaimCeiling::SupportsCompleteCoverageClaim
        } else {
            ClaimCeiling::WithheldOrBlocked
        }),
        supported_claim_classes: Some(if complete {
            vec![SupportedClaimClass::CompleteCoverage]
        } else {
            Vec::new()
        }),
        blocked_claim_classes: Some(strict_blocked()),
        cargo_version: None,
        rustc_version: None,
        flags: None,
        coverage_cache_class: None,
        boundary_lineage: None,
        boundary_lineage_digest: None,
        strict_boundary_authority_status: None,
        strict_boundary_required_for_claims: None,
        equivalence_status: None,
        current_candidate_digest: None,
        strict_boundary_receipt_path: None,
        strict_boundary_receipt_digest: None,
    };
    let resolved = crate::output_path::claim_artifact_path(root, receipt, "coverage receipt")?;
    write_receipt(&resolved, &record).map_err(|error| error.to_string())
}

fn claim_path(root: &Path, relative: &str, label: &str) -> Result<std::path::PathBuf, String> {
    let confined = crate::output_path::claim_artifact_path(root, Path::new(relative), label)?;
    confined
        .canonicalize()
        .or_else(|_| Ok(root.join(relative)))
        .map_err(|error: std::io::Error| error.to_string())
}

fn tool_version(tool: CoverageTool) -> Result<super::contract_codec::ToolVersion, String> {
    match coverage_tool_effect_adapter::execute(CoverageToolEffectRequest::ReadVersion { tool })
        .map_err(|error| error.to_string())?
    {
        CoverageToolEffectResponse::Version(version) => Ok(version),
        _ => Err("coverage_tool_version_response_mismatch".to_string()),
    }
}

fn workspace_root(root: &Path) -> String {
    root.canonicalize()
        .unwrap_or_else(|_| root.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn strict_blocked() -> Vec<BlockedClaimClass> {
    use BlockedClaimClass::*;
    vec![
        Completion,
        PackageReadiness,
        ReviewReadiness,
        ReleaseReadiness,
        FinalPacketCorrectness,
        UpdateGoalEligibility,
        AppRegistryOrReviewerExposure,
    ]
}
