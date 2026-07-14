use super::*;

pub(crate) const MAX_SELECTED_CHECKS: usize = 4_096;
pub(crate) const MAX_ARGUMENTS: usize = 128;
pub(crate) const MAX_ARGUMENT_BYTES: usize = 64 * 1024;
pub(crate) const MAX_ENVIRONMENT_ENTRIES: usize = 64;
pub(crate) const MAX_ENVIRONMENT_BYTES: usize = 64 * 1024;
pub(crate) const MAX_READ_SOURCES: usize = 128;
pub(crate) const MAX_OUTPUT_SCOPES: usize = 128;
pub(crate) const MAX_TIMEOUT_MS: u64 = 3_600_000;
pub(crate) const MAX_OUTPUT_BUDGET_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const REQUEST_DOMAIN: &[u8] = b"routine-effect-request-v1";
pub(crate) const REQUEST_SEAL_DOMAIN: &[u8] = b"routine-effect-request-seal-v1";

pub(crate) static NEXT_REQUEST_ISSUANCE: AtomicU64 = AtomicU64::new(1);

#[derive(Serialize)]
pub(crate) struct BoundIntent {
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

#[derive(Serialize)]
pub(crate) struct IntentCommitment<'a> {
    pub(crate) intent_id: &'a str,
    pub(crate) intent: &'a BoundIntent,
}

#[derive(Serialize)]
pub(crate) struct ProtocolPayload<'a> {
    pub(crate) binding_id: &'a str,
    pub(crate) graph_id: &'a str,
    pub(crate) snapshot_id: &'a str,
    pub(crate) plan_id: &'a str,
    pub(crate) result_scope: &'a str,
    pub(crate) intents: &'a [IntentCommitment<'a>],
}

#[derive(Serialize)]
pub(crate) struct NoOpPayload<'a> {
    pub(crate) binding_id: &'a str,
    pub(crate) graph_id: &'a str,
    pub(crate) snapshot_id: &'a str,
    pub(crate) plan_id: &'a str,
    pub(crate) result_scope: &'a str,
    pub(crate) selected: &'a [String],
    pub(crate) effect_intent_count: usize,
    pub(crate) support_limit: &'static str,
}

pub(crate) struct RunnerIdentity {
    pub(crate) tool_name: String,
    pub(crate) tool_identity_sha256: String,
    pub(crate) program_path_hex: String,
    pub(crate) program_sha256: String,
    pub(crate) program_byte_length: u64,
    pub(crate) program_unix_mode: Option<u32>,
    pub(crate) program_path: String,
}

pub(crate) fn bind_routine_invocation(
    context: &LiveContext,
    plan: &RoutinePlan,
    node_id: &str,
    arguments: Vec<String>,
    timeout_ms: u64,
    output_budget_bytes: u64,
    declared_output_scopes: Vec<RepoPath>,
) -> Result<RoutineInvocationSpec, RoutineError> {
    let binding = structural_binding(context, plan, "adapter-invocation-plan-binding-is-stale")?;
    let check = plan
        .check(node_id)
        .ok_or_else(|| adapter_error("adapter-invocation-node-unknown"))?;
    let runner = exact_runner(context, check)?;
    let environment = default_environment(&runner)?;
    bind_routine_invocation_with_environment_inner(
        binding,
        check,
        runner,
        arguments,
        environment,
        Vec::new(),
        timeout_ms,
        output_budget_bytes,
        declared_output_scopes,
    )
}

pub(crate) fn bind_routine_invocation_with_environment(
    context: &LiveContext,
    plan: &RoutinePlan,
    node_id: &str,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    timeout_ms: u64,
    output_budget_bytes: u64,
    declared_output_scopes: Vec<RepoPath>,
) -> Result<RoutineInvocationSpec, RoutineError> {
    let binding = structural_binding(context, plan, "adapter-invocation-plan-binding-is-stale")?;
    let check = plan
        .check(node_id)
        .ok_or_else(|| adapter_error("adapter-invocation-node-unknown"))?;
    let runner = exact_runner(context, check)?;
    bind_routine_invocation_with_environment_inner(
        binding,
        check,
        runner,
        arguments,
        environment,
        Vec::new(),
        timeout_ms,
        output_budget_bytes,
        declared_output_scopes,
    )
}

pub(crate) fn bind_routine_invocation_with_read_sources(
    context: &LiveContext,
    plan: &RoutinePlan,
    node_id: &str,
    arguments: Vec<String>,
    read_sources: Vec<RepoPath>,
    timeout_ms: u64,
    output_budget_bytes: u64,
    declared_output_scopes: Vec<RepoPath>,
) -> Result<RoutineInvocationSpec, RoutineError> {
    let binding = structural_binding(context, plan, "adapter-invocation-plan-binding-is-stale")?;
    let check = plan
        .check(node_id)
        .ok_or_else(|| adapter_error("adapter-invocation-node-unknown"))?;
    let runner = exact_runner(context, check)?;
    let environment = default_environment(&runner)?;
    bind_routine_invocation_with_environment_inner(
        binding,
        check,
        runner,
        arguments,
        environment,
        read_sources,
        timeout_ms,
        output_budget_bytes,
        declared_output_scopes,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn bind_routine_invocation_with_environment_and_read_sources(
    context: &LiveContext,
    plan: &RoutinePlan,
    node_id: &str,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    read_sources: Vec<RepoPath>,
    timeout_ms: u64,
    output_budget_bytes: u64,
    declared_output_scopes: Vec<RepoPath>,
) -> Result<RoutineInvocationSpec, RoutineError> {
    let binding = structural_binding(context, plan, "adapter-invocation-plan-binding-is-stale")?;
    let check = plan
        .check(node_id)
        .ok_or_else(|| adapter_error("adapter-invocation-node-unknown"))?;
    let runner = exact_runner(context, check)?;
    bind_routine_invocation_with_environment_inner(
        binding,
        check,
        runner,
        arguments,
        environment,
        read_sources,
        timeout_ms,
        output_budget_bytes,
        declared_output_scopes,
    )
}
