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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorKind {
    Agent,
    Human,
    AgentWithHumanSupervision,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Intent,
    Research,
    Prototype,
    Source,
    Package,
    Installed,
    Runtime,
    AgentUse,
    HumanUse,
    RepeatedHumanUse,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceStatus {
    Observed,
    Withheld,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceIdentity {
    pub status: SurfaceStatus,
    pub identity: Option<String>,
    pub evidence: Option<EvidenceBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceIdentities {
    pub source: SurfaceIdentity,
    pub package: SurfaceIdentity,
    pub marketplace: SurfaceIdentity,
    pub install: SurfaceIdentity,
    pub cache: SurfaceIdentity,
    pub app_registry: SurfaceIdentity,
    pub discovery: SurfaceIdentity,
    pub runtime: SurfaceIdentity,
    pub journey: SurfaceIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicEntryObservation {
    pub surface_id: String,
    pub route: String,
    pub bypass_attempted: bool,
    pub bypass_rejected: bool,
    pub evidence: EvidenceBinding,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealWorkObservation {
    pub repository_identity: String,
    pub repository_evidence: EvidenceBinding,
    pub task_id: String,
    pub task: String,
    pub useful_outcome: String,
    pub evidence: EvidenceBinding,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManualJourneyRow {
    pub time_to_verified_value_ms: u64,
    pub human_interventions: u64,
    pub review_rounds: u64,
    pub failure: String,
    pub diagnosis: String,
    pub recovery_outcome: String,
    pub repeat_use_outcome: String,
    pub retained_artifact_bytes: u64,
    pub retained_cache_bytes: u64,
    pub false_passes: u64,
    pub false_rejections: u64,
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
    #[serde(default)]
    pub operator_kind: Option<OperatorKind>,
    #[serde(default)]
    pub evidence_class: Option<EvidenceClass>,
    #[serde(default)]
    pub surface_identities: Option<SurfaceIdentities>,
    #[serde(default)]
    pub public_entry_observation: Option<PublicEntryObservation>,
    #[serde(default)]
    pub real_work_observation: Option<RealWorkObservation>,
    #[serde(default)]
    pub manual_journey_row: Option<ManualJourneyRow>,
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
    MissingV2Field,
    MissingTruthLayer,
    MissingObservation,
    BypassAttempt,
    AgentUseRelabeledHumanUse,
    AgentUseRelabeledContinuance,
    ContinuanceRequiresRepeatedHumanUse,
    InvalidObservation,
    InvalidRepository,
    InvalidTask,
    WrongSurface,
    UnsupportedClaimCeiling,
    ReviewerAuthorityInvalid,
    TruthLayerEscalation,
}
