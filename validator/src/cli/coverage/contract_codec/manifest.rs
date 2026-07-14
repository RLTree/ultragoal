use super::scalar::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CoverageManifest {
    pub(crate) schema: SchemaId,
    pub(crate) manifest_version: u64,
    pub(crate) repo_root_digest: Digest,
    pub(crate) generated_at: GeneratedAt,
    pub(crate) owner: Owner,
    pub(crate) policy: CoveragePolicy,
    pub(crate) coverage_command_id: CoverageCommandId,
    pub(crate) coverage_command_path: CoverageCommandPath,
    pub(crate) coverage_receipt_output_path: RepositoryPath,
    pub(crate) source_discovery_rules: SourceDiscoveryRules,
    pub(crate) repo_owned_source_roots: Vec<RepositoryPath>,
    pub(crate) generated_roots: Vec<RepositoryPath>,
    pub(crate) vendor_roots: Vec<RepositoryPath>,
    pub(crate) external_roots: Vec<RepositoryPath>,
    pub(crate) required_measured_dimensions_per_root: Vec<MeasuredRoot>,
    pub(crate) required_target_paths: Vec<RepositoryPath>,
    pub(crate) exclusions: Vec<CoverageExclusion>,
    pub(crate) changed_file_coupling_policy: ChangedFileCouplingPolicy,
    pub(crate) repo_walk_policy: BTreeMap<String, bool>,
    pub(crate) behavior_dimension_mapping: BTreeMap<String, Vec<CoverageDimension>>,
    pub(crate) policy_mutation_gate: BTreeMap<String, bool>,
    pub(crate) receipt_freshness_binding: BTreeMap<String, bool>,
    pub(crate) tool_generated_proof_policy: BTreeMap<String, bool>,
    pub(crate) fast_full_gate_split: FastFullGateSplit,
    pub(crate) claim_ceiling_when_incomplete: ClaimCeiling,
}

impl CoverageManifest {
    pub(crate) fn target_paths(&self) -> Vec<String> {
        self.required_target_paths
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    pub(crate) fn measured_dimensions(&self) -> Vec<CoverageDimension> {
        let mut dimensions = self
            .required_measured_dimensions_per_root
            .iter()
            .flat_map(|row| row.dimensions.iter().copied())
            .collect::<Vec<_>>();
        dimensions.sort();
        dimensions.dedup();
        dimensions
    }

    pub(crate) fn changed_files(&self) -> Vec<String> {
        self.changed_file_coupling_policy
            .changed_files
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    pub(crate) fn ignore_patterns(&self) -> Vec<String> {
        self.source_discovery_rules
            .ignore
            .iter()
            .map(ToString::to_string)
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceDiscoveryRules {
    pub(crate) include: Vec<RepositoryPath>,
    pub(crate) ignore: Vec<RepositoryPath>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MeasuredRoot {
    pub(crate) root: RepositoryPath,
    pub(crate) dimensions: Vec<CoverageDimension>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CoverageExclusion {
    pub(crate) path: RepositoryPath,
    pub(crate) kind: ExclusionKind,
    pub(crate) rationale: Reason,
    pub(crate) reviewed: bool,
    pub(crate) counts_as_covered: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChangedFileCouplingPolicy {
    pub(crate) required: bool,
    pub(crate) changed_files: Vec<RepositoryPath>,
    pub(crate) changed_files_digest: Digest,
    pub(crate) blocker_path: RepositoryPath,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FastFullGateSplit {
    pub(crate) fast_gate: RepositoryPath,
    pub(crate) full_gate: RepositoryPath,
    pub(crate) fast_supports_completion: bool,
    pub(crate) full_required_for_completion: bool,
    pub(crate) missing_claim_context_fails: bool,
}
