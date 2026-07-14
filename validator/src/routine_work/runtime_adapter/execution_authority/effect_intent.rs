use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RoutineEffectIntent {
    pub(crate) protocol_id: String,
    pub(crate) intent_id: String,
    pub(crate) plan_order: usize,
    pub(crate) node_id: String,
    pub(crate) selected_tool: String,
    pub(crate) tool_identity_sha256: String,
    pub(crate) program_path_hex: String,
    pub(crate) program_sha256: String,
    pub(crate) program_byte_length: u64,
    pub(crate) program_unix_mode: Option<u32>,
    pub(crate) argv: Vec<String>,
    pub(crate) working_directory: String,
    pub(crate) environment_policy: &'static str,
    pub(crate) environment_keys: Vec<String>,
    pub(crate) environment_sha256: String,
    #[serde(skip)]
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) read_authority_policy: &'static str,
    pub(crate) read_source_paths: Vec<RepoPath>,
    pub(crate) read_authority_sha256: String,
    #[serde(skip)]
    pub(crate) read_sources: Vec<RoutineReadSource>,
    pub(crate) mediation_preflight: &'static str,
    pub(crate) timeout_ms: u64,
    pub(crate) output_budget_bytes: u64,
    pub(crate) declared_output_scopes: Vec<RepoPath>,
    pub(crate) expected_dependency_nodes: Vec<String>,
    pub(crate) input_id: String,
}

impl RoutineEffectIntent {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        protocol_id: String,
        intent_id: String,
        plan_order: usize,
        node_id: String,
        selected_tool: String,
        tool_identity_sha256: String,
        program_path_hex: String,
        program_sha256: String,
        program_byte_length: u64,
        program_unix_mode: Option<u32>,
        argv: Vec<String>,
        working_directory: String,
        environment_sha256: String,
        environment: BTreeMap<String, String>,
        read_authority_sha256: String,
        read_sources: Vec<RoutineReadSource>,
        timeout_ms: u64,
        output_budget_bytes: u64,
        declared_output_scopes: Vec<RepoPath>,
        expected_dependency_nodes: Vec<String>,
        input_id: String,
    ) -> Self {
        Self {
            protocol_id,
            intent_id,
            plan_order,
            node_id,
            selected_tool,
            tool_identity_sha256,
            program_path_hex,
            program_sha256,
            program_byte_length,
            program_unix_mode,
            argv,
            working_directory,
            environment_policy: "clear-all-allowlisted-v1",
            environment_keys: environment.keys().cloned().collect(),
            environment_sha256,
            environment,
            read_authority_policy: "default-deny-exact-bound-read-v1",
            read_source_paths: read_sources
                .iter()
                .map(|source| source.relative_path.clone())
                .collect(),
            read_authority_sha256,
            read_sources,
            mediation_preflight: "revalidate-context-candidate-tool-executable-read-sources-output-scopes-before-and-after-effect-v1",
            timeout_ms,
            output_budget_bytes,
            declared_output_scopes,
            expected_dependency_nodes,
            input_id,
        }
    }

    pub(crate) fn protocol_id(&self) -> &str {
        &self.protocol_id
    }
    pub(crate) fn intent_id(&self) -> &str {
        &self.intent_id
    }
    pub(crate) fn plan_order(&self) -> usize {
        self.plan_order
    }
    pub(crate) fn node_id(&self) -> &str {
        &self.node_id
    }
    pub(crate) fn selected_tool(&self) -> &str {
        &self.selected_tool
    }
    pub(crate) fn tool_identity_sha256(&self) -> &str {
        &self.tool_identity_sha256
    }
    pub(crate) fn program_path_hex(&self) -> &str {
        &self.program_path_hex
    }
    pub(crate) fn program_sha256(&self) -> &str {
        &self.program_sha256
    }
    pub(crate) fn program_byte_length(&self) -> u64 {
        self.program_byte_length
    }
    pub(crate) fn program_unix_mode(&self) -> Option<u32> {
        self.program_unix_mode
    }
    pub(crate) fn argv(&self) -> &[String] {
        &self.argv
    }
    pub(crate) fn working_directory(&self) -> &str {
        &self.working_directory
    }
    pub(crate) fn environment_policy(&self) -> &'static str {
        self.environment_policy
    }
    pub(crate) fn environment_keys(&self) -> &[String] {
        &self.environment_keys
    }
    pub(crate) fn environment_sha256(&self) -> &str {
        &self.environment_sha256
    }
    pub(crate) fn environment(&self) -> &BTreeMap<String, String> {
        &self.environment
    }
    pub(crate) fn read_authority_policy(&self) -> &'static str {
        self.read_authority_policy
    }
    pub(crate) fn read_source_paths(&self) -> &[RepoPath] {
        &self.read_source_paths
    }
    pub(crate) fn read_authority_sha256(&self) -> &str {
        &self.read_authority_sha256
    }
    pub(crate) fn read_sources(&self) -> &[RoutineReadSource] {
        &self.read_sources
    }
    pub(crate) fn mediation_preflight(&self) -> &'static str {
        self.mediation_preflight
    }
    pub(crate) fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }
    pub(crate) fn output_budget_bytes(&self) -> u64 {
        self.output_budget_bytes
    }
    pub(crate) fn declared_output_scopes(&self) -> &[RepoPath] {
        &self.declared_output_scopes
    }
    pub(crate) fn expected_dependency_nodes(&self) -> &[String] {
        &self.expected_dependency_nodes
    }
    pub(crate) fn input_id(&self) -> &str {
        &self.input_id
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RoutineNoOpProjection {
    pub(crate) projection_id: String,
    pub(crate) binding_id: String,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) graph_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) plan_id: String,
    pub(crate) result_scope: String,
    pub(crate) selected: Vec<String>,
    pub(crate) status: ReportStatus,
    pub(crate) effect_intent_count: usize,
    pub(crate) support_limit: &'static str,
}
