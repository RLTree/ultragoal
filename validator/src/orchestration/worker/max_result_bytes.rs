const MAX_RESULT_BYTES: usize = 4 * 1024 * 1024;
pub const NO_CLAIM: &str = "This worker does not claim readiness, release, or completion.";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectUse {
    pub class: EffectClass,
    pub target: String,
    pub performed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerResultV1 {
    pub worker: String,
    pub lease_id: String,
    pub context_id: String,
    pub candidate_identity: BTreeMap<String, Value>,
    pub base_state: BTreeMap<String, Value>,
    pub final_state: BTreeMap<String, Value>,
    pub touched_paths: Vec<String>,
    pub touched_semantics: Vec<String>,
    pub generated_outputs: Vec<String>,
    pub fixtures: Vec<String>,
    pub effects: Vec<EffectUse>,
    pub requirements: Vec<String>,
    pub dependency_nodes: Vec<String>,
    pub changes: Vec<BTreeMap<String, Value>>,
    pub commands_and_tests: Vec<BTreeMap<String, Value>>,
    pub artifacts: Vec<ArtifactRecord>,
    pub findings: Vec<BTreeMap<String, Value>>,
    pub unresolved_dependencies: Vec<String>,
    pub requested_root_changes: Vec<RootChangeRequest>,
    pub limitations: Vec<String>,
    pub no_claim_statement: String,
}
