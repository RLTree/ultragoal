use serde::{Deserialize, Serialize};

pub const VERSION: &str = "0.1.0";

pub use crate::contract_check_ids::CHECK_IDS;

pub const REQUIRED_SKILLS: &[&str] = &[
    "ultragoal",
    "harness-engineering",
    "agent-first-repo-init",
    "agent-first-repo-retrofit",
    "agent-runtime-legibility",
    "agent-observability-stack",
    "product-cohesion-gate",
    "product-fitness-gate",
    "execplan-lane",
    "orchestrator-reconciler",
    "proof-gate",
    "standards-gardener",
];

pub const REQUIRED_AGENTS: &[&str] = &[
    "harness-contract-claim-falsifier",
    "harness-orchestration-recovery-falsifier",
    "harness-security-trust-boundary-falsifier",
    "harness-product-simplicity-falsifier",
    "harness-material-review-scope-gatekeeper",
    "harness-repo-initializer",
    "harness-retrofit-planner",
];

pub const GOOD_STATUSES: &[&str] = &["proven_live", "proven_static"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SemanticClass {
    ProductUserFacingSurface,
    UiControlSurface,
    LocalAppDesktopBrowser,
    Dashboard,
    RunConsole,
    WorkflowLauncher,
    InstallVisibleSelectableActiveInCodex,
    PublicationMarketplaceCatalogWorkspaceRegistry,
    RuntimeCliBackendOnlyEngineOnly,
    AmbiguousNeedsReviewerClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ProofGate {
    ProductCohesionReceipt,
    UiJourneyEvidence,
    AccessibilityEvidence,
    RuntimeExecution,
    LiveBeneficialE2e,
    InstallVisibilityReceipt,
    PublicationExternalAttestation,
    ReadyForMergeReceipt,
    ObservabilityReceipt,
    ReviewerClassification,
    WithheldOrBacklogClaim,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClassifierKind {
    Model,
    HumanReviewer,
    DeterministicBackstop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    pub provider: String,
    pub model: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifierEvidence {
    pub evidence_type: String,
    pub digest: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticClassificationReceipt {
    pub schema: String,
    pub claim_id: String,
    pub canonical_text_digest: String,
    pub classifier_contract_id: String,
    pub classifier_contract_version: String,
    pub classifier_implementation_kind: ClassifierKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_model: Option<ProviderModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_contract_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classifier_evidence: Option<ClassifierEvidence>,
    pub generated_at: String,
    pub producer_actor_id: String,
    pub classifier_actor_id: String,
    pub actor_disjoint: bool,
    pub detected_semantic_classes: Vec<SemanticClass>,
    pub rationale: String,
    pub confidence: f64,
    pub ambiguity: bool,
    pub required_proof_gates: Vec<ProofGate>,
    pub claim_ceiling_recommendation: String,
    pub receipt_digest: String,
}

#[derive(Debug, Clone)]
pub struct Failure {
    pub check_id: String,
    pub error: String,
    pub detail: String,
}

impl Failure {
    pub fn new(check_id: &str, error: &str, detail: impl Into<String>) -> Self {
        Self {
            check_id: check_id.to_string(),
            error: error.to_string(),
            detail: detail.into(),
        }
    }
}
