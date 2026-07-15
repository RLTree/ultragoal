use super::*;

pub(crate) fn bind_public_invocation(
    context: &LiveContext,
    manifest: &LoadedManifest,
    plan: &RoutinePlan,
    invocation: &BoundCatalogInvocation,
) -> Result<RoutineInvocationSpec, PublicFailure> {
    let node = manifest
        .node(invocation.node_id())
        .ok_or(PublicFailure::Catalog("routine-public-bound-node-unknown"))?;
    let check = plan
        .check(invocation.node_id())
        .ok_or(PublicFailure::Catalog(
            "routine-public-bound-plan-node-unknown",
        ))?;
    let executable = safe_executable(
        context
            .capabilities()
            .tool(invocation.selected_tool())
            .ok_or(PublicFailure::Catalog(
                "routine-public-bound-runner-unobserved",
            ))?,
        invocation.selected_tool(),
    )?;
    let expected_environment =
        crate::routine_work::fixed_environment(&executable).map_err(PublicFailure::Routine)?;
    let expected_reads = node
        .read_sources
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let observed_reads = invocation
        .read_sources()
        .iter()
        .map(|source| source.relative_path())
        .collect::<Vec<_>>();
    let expected_output = node.output_scope();
    if invocation.behavior_id() != manifest::ROUTINE_BEHAVIOR
        || invocation.selected_tool() != check.selected_tool()
        || invocation.arguments() != node.canonical_arguments()
        || invocation.environment() != &expected_environment
        || observed_reads != expected_reads
        || invocation.timeout_ms() != node.timeout_ms
        || invocation.output_budget_bytes() != node.output_budget_bytes
        || invocation.output_scopes().len() != 1
        || invocation.output_scopes()[0].relative_path() != expected_output
    {
        return Err(PublicFailure::Catalog(
            "routine-public-catalog-policy-mismatch",
        ));
    }
    let reads = node
        .read_sources
        .iter()
        .map(RepoPath::parse)
        .collect::<Result<Vec<_>, _>>()
        .map_err(PublicFailure::Routine)?;
    let outputs = vec![RepoPath::parse(expected_output).map_err(PublicFailure::Routine)?];
    bind_rust_source_syntax_invocation(
        context,
        plan,
        invocation.node_id(),
        reads,
        invocation.timeout_ms(),
        invocation.output_budget_bytes(),
        outputs,
    )
    .map_err(PublicFailure::Routine)
}
