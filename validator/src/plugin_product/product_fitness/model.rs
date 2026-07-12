use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitnessDimension {
    Accessibility,
    CognitiveLoad,
    RecoveryBurden,
    Continuance,
    RealUseEvidence,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DimensionDisposition {
    Pass,
    Fail,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TruthLayer {
    Source,
    Package,
    Marketplace,
    Install,
    Cache,
    AppRegistry,
    PluginsUi,
    Discovery,
    Runtime,
    Journey,
}

pub const TRUTH_LAYERS: [TruthLayer; 10] = [
    TruthLayer::Source,
    TruthLayer::Package,
    TruthLayer::Marketplace,
    TruthLayer::Install,
    TruthLayer::Cache,
    TruthLayer::AppRegistry,
    TruthLayer::PluginsUi,
    TruthLayer::Discovery,
    TruthLayer::Runtime,
    TruthLayer::Journey,
];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimCeiling {
    CandidateOnly,
    Withheld,
    Blocked,
    LiveSameSurfaceProven,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBinding {
    pub path: String,
    pub sha256: String,
    pub candidate_id: String,
    pub same_surface: bool,
    pub current_session: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionEvidence {
    pub dimension: FitnessDimension,
    pub disposition: DimensionDisposition,
    pub finding: String,
    pub evidence: EvidenceBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverallDisposition {
    Pass,
    Fail,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductFitnessDisposition {
    pub schema_version: String,
    pub candidate_id: String,
    pub producer_actor_id: String,
    pub reviewer_actor_id: String,
    pub owner_role: String,
    pub reviewer_authority: String,
    pub may_raise_claim_ceiling: bool,
    pub dimensions: Vec<DimensionEvidence>,
    pub truth_layer_ceilings: BTreeMap<TruthLayer, ClaimCeiling>,
    pub substitution_rejections: BTreeSet<String>,
    pub overall: OverallDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductFitnessError {
    ActorNotDisjoint,
    CandidateMismatch,
    DuplicateDimension,
    EvidenceDigestMismatch,
    EvidenceMissing,
    EvidencePathInvalid,
    EvidenceSpecialFile,
    EvidenceStale,
    InvalidActor,
    InvalidDigest,
    InvalidDisposition,
    MissingDimension,
    MissingSubstitutionRejection,
    MissingTruthLayer,
    ReviewerAuthorityInvalid,
    TruthLayerEscalation,
}
