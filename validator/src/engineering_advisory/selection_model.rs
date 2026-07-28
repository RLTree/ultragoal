use crate::plugin_product::skill_catalog::{AgenticCoInstallProfile, SkillCatalogProjection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryLens {
    AgenticEngineering,
    CodexTaskContract,
    ContextRepositoryEngineering,
    AgentProductDiscovery,
    ConceptFeasibilityValidation,
    AuthenticUseEngineering,
    RequirementsSystemsEngineering,
    AgenticProductLifecycle,
    ArchitectureDeliveryPlanning,
    AgentFirstSoftwareEngineering,
    SoftwareConstructionQuality,
    HarnessEngineering,
    LoopEngineering,
    GraphWorkflowEngineering,
    MultiAgentEngineering,
    VerificationStrategyEngineering,
    AgentEvalsObservability,
    AgentSecurityGovernance,
    RustAgenticArchitecture,
    RustAgentRuntime,
    RustAgentDurability,
    RustAgentProtocols,
    RustAgentVerification,
    RustAgentObservability,
    ProductFitnessEngineering,
    ContinuousProductExperimentation,
    SecureDeliveryRelease,
    ProductionReadinessSre,
    MaintenanceRetirementEngineering,
    EngineeringLearningLoop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisoryOutcomeClass {
    Routine,
    AlreadySpecified,
    NoChange,
    MaterialDecision,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvisorySelectionDisposition {
    NoAdvisoryNeeded,
    AdvisorySelected,
    MaterialInputMissing,
    RequiredProfileUnavailable,
    StaleOrCrossCandidate,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvisorySelectionRequest {
    pub candidate_id: String,
    pub context_id: String,
    pub product_state_id: String,
    pub config_digest: String,
    pub user_outcome_class: AdvisoryOutcomeClass,
    pub explicit_lens_requests: BTreeSet<AdvisoryLens>,
    pub lifecycle_stage: String,
    pub active_truth_loop_id: Option<String>,
    pub first_broken_transition_id: Option<String>,
    pub protected_invariant_ids: BTreeSet<String>,
    pub contemplated_effects: BTreeSet<String>,
    pub authority_gaps: BTreeSet<String>,
    pub material_uncertainties: BTreeSet<String>,
    pub verification_oracle: Option<String>,
    pub false_pass_risks: BTreeSet<String>,
    pub failure_classes: BTreeSet<String>,
    pub recovery_ambiguities: BTreeSet<String>,
    pub product_fitness_gaps: BTreeSet<String>,
    pub activation_signals: BTreeSet<AdvisoryLens>,
    pub plugin_version: String,
    pub plugin_digest: String,
    pub profile_digest: String,
    pub selector_version: String,
    pub profile: Option<AgenticCoInstallProfile>,
    pub catalog: Option<SkillCatalogProjection>,
    pub prior_selection: Option<Box<EngineeringAdvisorySelection>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineeringAdvisorySelection {
    pub schema_version: String,
    pub selection_id: String,
    pub input_fingerprint: String,
    pub candidate_id: String,
    pub context_id: String,
    pub disposition: AdvisorySelectionDisposition,
    pub primary_lens: Option<AdvisoryLens>,
    pub supporting_lenses: Vec<AdvisoryLens>,
    pub activation_reasons: Vec<String>,
    pub assumptions: BTreeSet<String>,
    pub missing_inputs: BTreeSet<String>,
    pub unsupported_surfaces: BTreeSet<String>,
    pub adopting_owner: String,
    pub invalidation_conditions: BTreeSet<String>,
    pub plain_language_result: String,
    pub plain_language_next_action: String,
    pub claim_ceiling: String,
    pub no_claim_statement: String,
}
