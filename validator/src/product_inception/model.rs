use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BriefV1 {
    pub(crate) schema: String,
    pub(crate) product_success_contract_id: String,
    pub(crate) claim_ids: Vec<String>,
    pub(crate) target_problem: String,
    pub(crate) audience: String,
    pub(crate) job_to_be_done: String,
    pub(crate) context_of_use: String,
    pub(crate) desired_outcome: String,
    pub(crate) first_value_event: String,
    pub(crate) evidence_ladder: String,
    pub(crate) claim_ceiling: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BriefV2 {
    pub(crate) schema: String,
    pub(crate) product_success_contract_id: String,
    pub(crate) product_success_contract_digest: String,
    pub(crate) claim_ids: Vec<String>,
    pub(crate) target_problem: String,
    pub(crate) audience: String,
    pub(crate) job_to_be_done: String,
    pub(crate) context_of_use: String,
    pub(crate) desired_outcome: String,
    pub(crate) first_value_event: String,
    pub(crate) operator: Operator,
    pub(crate) real_work: RealWork,
    pub(crate) public_entry_surface: PublicEntrySurface,
    pub(crate) protected_invariants: Vec<ProtectedInvariant>,
    pub(crate) first_truth_loop: TruthLoop,
    pub(crate) depth_triggers: Vec<DepthTrigger>,
    pub(crate) evidence_class: EvidenceClass,
    pub(crate) evidence_ladder: String,
    pub(crate) claim_ceiling: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Operator {
    pub(crate) kind: OperatorKind,
    pub(crate) actor_reference: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OperatorKind {
    Agent,
    Human,
    AgentWithHumanSupervision,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RealWork {
    pub(crate) repository_identity: String,
    pub(crate) starting_candidate: String,
    pub(crate) dirty_state_expectation: DirtyStateExpectation,
    pub(crate) task_id: String,
    pub(crate) task: String,
    pub(crate) expected_useful_outcome: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DirtyStateExpectation {
    Clean,
    Dirty,
    Either,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicEntrySurface {
    pub(crate) surface_id: String,
    pub(crate) route: String,
    pub(crate) forbidden_bypasses: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProtectedInvariant {
    pub(crate) id: String,
    pub(crate) claim_ids: Vec<String>,
    pub(crate) surfaces: Vec<String>,
    pub(crate) required_condition: String,
    pub(crate) disposition: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TruthLoop {
    pub(crate) loop_id: String,
    pub(crate) positive_path: Vec<Transition>,
    pub(crate) first_value_transition: String,
    pub(crate) failure_control: FailureControl,
    pub(crate) preservation_expectation: String,
    pub(crate) repeat_use_expectation: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Transition {
    pub(crate) transition_id: String,
    pub(crate) order: u32,
    pub(crate) dependency_ids: Vec<String>,
    pub(crate) capability_ids: Vec<String>,
    pub(crate) claim_ids: Vec<String>,
    pub(crate) product_surfaces: Vec<String>,
    pub(crate) expected_observation: String,
    pub(crate) evidence_class: EvidenceClass,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FailureControl {
    pub(crate) transition_id: String,
    pub(crate) failure: String,
    pub(crate) diagnosis: String,
    pub(crate) recovery: String,
    pub(crate) preservation: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DepthTrigger {
    pub(crate) trigger_id: String,
    pub(crate) kind: DepthTriggerKind,
    pub(crate) risk_or_claim: String,
    pub(crate) activation_finding_codes: Vec<String>,
    pub(crate) smallest_investment: String,
    pub(crate) fitness_function: String,
    pub(crate) invalidation_condition: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DepthTriggerKind {
    ProtectedInvariant,
    ObservedFailure,
    RepeatedGap,
    BoundedExperiment,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceClass {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContractFacts {
    pub(crate) product_contract_id: String,
    pub(crate) contract_version: String,
    pub(crate) contract_digest: String,
    pub(crate) authority_contract_id: String,
    pub(crate) claim_registry_digest: String,
    pub(crate) claim_ids: Vec<String>,
    pub(crate) public_surface_catalog_digest: String,
    pub(crate) surface_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CandidateBinding {
    pub(crate) head_commit: Option<String>,
    pub(crate) head_tree: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) dirty: bool,
    pub(crate) candidate_digest: String,
    pub(crate) repository_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ContractBinding {
    pub(crate) product_success_contract_id: String,
    pub(crate) contract_version: String,
    pub(crate) product_success_contract_digest: String,
    pub(crate) authority_contract_id: String,
    pub(crate) claim_registry_digest: String,
    pub(crate) public_surface_catalog_digest: String,
}
