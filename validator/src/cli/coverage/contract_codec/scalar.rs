use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! semantic_text {
    ($name:ident) => {
        #[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub(crate) struct $name(String);

        #[allow(dead_code)]
        impl $name {
            pub(crate) fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }

            pub(crate) fn is_empty(&self) -> bool {
                self.0.trim().is_empty()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

semantic_text!(ClaimId);
semantic_text!(CoverageCommandLine);
semantic_text!(CoverageCommandPath);
semantic_text!(CoverageCommandId);
semantic_text!(CoverageTargetDir);
semantic_text!(DebtId);
semantic_text!(Digest);
semantic_text!(GeneratedAt);
semantic_text!(Owner);
semantic_text!(Reason);
semantic_text!(RepositoryPath);
semantic_text!(SchemaId);
semantic_text!(ToolIdentity);
semantic_text!(ToolVersion);
semantic_text!(WorkspaceRoot);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CoveragePolicy {
    #[serde(rename = "100_percent_required")]
    HundredPercentRequired,
    RatchetFloor,
    BlockedOrWithheld,
    RoutineRepairFeedback,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CoverageDimension {
    Line,
    Branch,
    Function,
    Region,
    UiState,
    Artifact,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExclusionKind {
    Generated,
    Vendor,
    External,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetRevisionKind {
    GitCommit,
    PackageDigest,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ClaimCeiling {
    SupportsCompleteCoverageClaim,
    RatchetFloorOnly,
    WithheldOrBlocked,
    RoutineRepairOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SupportedClaimClass {
    CompleteCoverage,
    RoutineCoverageFeedback,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BlockedClaimClass {
    CompleteCoverage,
    Completion,
    PackageReadiness,
    ReviewReadiness,
    ReleaseReadiness,
    FinalPacketCorrectness,
    UpdateGoalEligibility,
    AppRegistryOrReviewerExposure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CoverageCacheClass {
    #[serde(rename = "retained_artifact_verified_local")]
    RetainedArtifactVerifiedLocal,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EquivalenceStatus {
    VerifiedCurrentInputEquivalent,
    RoutineFeedbackOnlyNoStrictBoundarySubstitution,
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StrictBoundaryAuthorityStatus {
    StrictBoundaryNotRun,
    StrictBoundaryCurrentInputEquivalent,
    #[serde(other)]
    Unknown,
}
