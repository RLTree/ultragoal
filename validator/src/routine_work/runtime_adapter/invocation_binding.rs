use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn bind_routine_invocation_with_environment_inner(
    binding: RoutineBinding,
    check: &PlannedCheck,
    runner: RunnerIdentity,
    behavior_id: String,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    read_source_paths: Vec<RepoPath>,
    timeout_ms: u64,
    output_budget_bytes: u64,
    declared_output_scopes: Vec<RepoPath>,
) -> Result<RoutineInvocationSpec, RoutineError> {
    let declared_output_scopes = normalized_output_scopes(declared_output_scopes)?;
    let read_source_paths = normalized_read_source_paths(read_source_paths)?;
    validate_execution_policy(
        &arguments,
        &environment,
        timeout_ms,
        output_budget_bytes,
        &declared_output_scopes,
    )?;
    if binding.worktree_root().to_str().is_none() {
        return Err(adapter_error("adapter-working-directory-not-utf8"));
    }
    let read_sources = mediator::bind_read_sources(binding.worktree_root(), &read_source_paths)?;
    let read_authority_sha256 = read_authority_digest(&read_sources)?;
    Ok(RoutineInvocationSpec::bound(
        check.node_id().to_owned(),
        behavior_id,
        runner.tool_name,
        runner.tool_identity_sha256,
        runner.program_path_hex,
        runner.program_sha256,
        runner.program_byte_length,
        runner.program_unix_mode,
        arguments,
        environment_digest(&environment)?,
        environment,
        read_authority_sha256,
        read_sources,
        timeout_ms,
        output_budget_bytes,
        declared_output_scopes,
    ))
}
