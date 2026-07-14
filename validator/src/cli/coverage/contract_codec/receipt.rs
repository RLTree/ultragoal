use super::manifest::CoverageExclusion;
use super::scalar::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CoverageReceipt {
    pub(crate) schema: Option<SchemaId>,
    pub(crate) claim_id: Option<ClaimId>,
    pub(crate) command: Option<CoverageCommandLine>,
    pub(crate) tool: Option<ToolIdentity>,
    pub(crate) coverage_target_dir: Option<CoverageTargetDir>,
    pub(crate) source_tree_digest: Option<Digest>,
    pub(crate) coverage_manifest_digest: Option<Digest>,
    pub(crate) coverage_command_digest: Option<Digest>,
    pub(crate) changed_files_digest: Option<Digest>,
    pub(crate) tool_version: Option<ToolVersion>,
    pub(crate) workspace_root: Option<WorkspaceRoot>,
    pub(crate) target_revision: Option<TargetRevision>,
    pub(crate) command_started_at: Option<GeneratedAt>,
    pub(crate) command_completed_at: Option<GeneratedAt>,
    pub(crate) command_exit: Option<i32>,
    pub(crate) machine_readable_report: Option<MachineReadableReport>,
    pub(crate) generated_by: Option<ToolIdentity>,
    pub(crate) percent_source: Option<ToolIdentity>,
    pub(crate) target_paths: Option<Vec<RepositoryPath>>,
    pub(crate) measured_dimensions: Option<Vec<CoverageDimension>>,
    pub(crate) coverage: Option<CoverageMeasurement>,
    pub(crate) uncovered_records: Option<Vec<UncoveredRecord>>,
    pub(crate) exclusions: Option<Vec<CoverageExclusion>>,
    pub(crate) generated_at: Option<GeneratedAt>,
    pub(crate) claim_ceiling: Option<ClaimCeiling>,
    pub(crate) supported_claim_classes: Option<Vec<SupportedClaimClass>>,
    pub(crate) blocked_claim_classes: Option<Vec<BlockedClaimClass>>,
    pub(crate) cargo_version: Option<ToolVersion>,
    pub(crate) rustc_version: Option<ToolVersion>,
    pub(crate) flags: Option<Vec<CoverageCommandLine>>,
    pub(crate) coverage_cache_class: Option<CoverageCacheClass>,
    pub(crate) boundary_lineage: Option<BoundaryLineage>,
    pub(crate) boundary_lineage_digest: Option<Digest>,
    pub(crate) strict_boundary_authority_status: Option<StrictBoundaryAuthorityStatus>,
    pub(crate) strict_boundary_required_for_claims: Option<Vec<BlockedClaimClass>>,
    pub(crate) equivalence_status: Option<EquivalenceStatus>,
    pub(crate) current_candidate_digest: Option<Digest>,
    pub(crate) strict_boundary_receipt_path: Option<RepositoryPath>,
    pub(crate) strict_boundary_receipt_digest: Option<Digest>,
    pub(crate) uncovered_count: Option<usize>,
}

impl CoverageReceipt {
    pub(crate) fn text<'a, T>(value: &'a Option<T>) -> &'a str
    where
        T: AsRef<str>,
    {
        value.as_ref().map(AsRef::as_ref).unwrap_or("")
    }

    pub(crate) fn target_paths(&self) -> Vec<String> {
        self.target_paths
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }
}

macro_rules! semantic_as_ref {
    ($($name:ty),+ $(,)?) => {
        $(impl AsRef<str> for $name {
            fn as_ref(&self) -> &str { self.as_str() }
        })+
    };
}

semantic_as_ref!(
    ClaimId,
    CoverageCommandLine,
    CoverageTargetDir,
    Digest,
    GeneratedAt,
    RepositoryPath,
    SchemaId,
    ToolIdentity,
    ToolVersion,
    WorkspaceRoot,
);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TargetRevision {
    pub(crate) kind: TargetRevisionKind,
    pub(crate) value: Digest,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MachineReadableReport {
    pub(crate) path: RepositoryPath,
    pub(crate) digest: Digest,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CoverageMeasurement {
    pub(crate) percent: f64,
    pub(crate) floor_percent: f64,
    pub(crate) policy: CoveragePolicy,
    pub(crate) owner: Option<Owner>,
    pub(crate) reason: Option<Reason>,
    pub(crate) blocker_or_debt_id: Option<DebtId>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UncoveredRecord {
    pub(crate) path: RepositoryPath,
    pub(crate) reason: Reason,
    pub(crate) owner: Option<Owner>,
    pub(crate) blocker_or_debt_id: Option<DebtId>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BoundaryLineage {
    pub(crate) mode: ClaimCeiling,
    pub(crate) strict_boundary_mode: ToolIdentity,
    pub(crate) source_tree_digest: Digest,
    pub(crate) coverage_manifest_digest: Digest,
    pub(crate) coverage_command_digest: Digest,
}
