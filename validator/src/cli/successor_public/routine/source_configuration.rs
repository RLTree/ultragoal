use super::*;

pub(crate) const SOURCE_CONFIG_KEY: &str = "contract_id";

pub(crate) fn execute(
    root: &Path,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    if let Some(outcome) = behavior_child::execute_if_requested(invocation) {
        return outcome;
    }
    execute_inner(root, invocation, home).unwrap_or_else(outcome::failure)
}

pub(crate) fn execute_inner(
    root: &Path,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> Result<RuntimeOutcome, PublicFailure> {
    let target = source_context::target_root(root, invocation)?;
    let discovery = source_context::discovery_context(&target)?;
    let manifest = manifest::load(&discovery, &target).map_err(PublicFailure::Manifest)?;
    let graph = manifest
        .graph()
        .map_err(|_| PublicFailure::Manifest(manifest::ManifestFailure::Invalid))?;
    let context = source_context::execution_context(&target, &manifest)?;
    validate_selected_sources(&context, &manifest)?;
    let snapshot = LocalDirtyTree::capture(&context).map_err(PublicFailure::Routine)?;
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine())
        .map_err(PublicFailure::Routine)?;
    let source_id = manifest.source_id();

    if plan.checks().is_empty() {
        let prepared = prepare(&context, &manifest, &graph, &snapshot, &plan)?;
        let result = mediate_public_routine_execution(
            None,
            &context,
            &plan,
            prepared,
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .map_err(PublicFailure::Routine)?;
        return Ok(outcome::mediation(
            &result,
            outcome::MediationContext {
                context_id: context.context_id(),
                candidate_id: plan.binding().candidate_id(),
                graph_id: graph.graph_id(),
                snapshot_id: snapshot.snapshot_id(),
                plan_id: plan.plan_id(),
                source_id: &source_id,
                fallback_tool_count: plan.affected_set().coverage().fallback_tool_count(),
            },
        ));
    }

    let prepared = prepare(&context, &manifest, &graph, &snapshot, &plan)?;
    let home = home.ok_or(PublicFailure::Host(host::HostFailure::Unavailable))?;
    let state = HostState::open(home).map_err(PublicFailure::Host)?;
    let mediated = mediate_public_routine_execution(
        Some(state.authority_root()),
        &context,
        &plan,
        prepared,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .map_err(PublicFailure::Routine);
    let result = mediated?;

    if state.verify().is_err() {
        return Err(PublicFailure::PersistenceAfterEffect);
    }
    Ok(outcome::mediation(
        &result,
        outcome::MediationContext {
            context_id: context.context_id(),
            candidate_id: plan.binding().candidate_id(),
            graph_id: graph.graph_id(),
            snapshot_id: snapshot.snapshot_id(),
            plan_id: plan.plan_id(),
            source_id: &source_id,
            fallback_tool_count: plan.affected_set().coverage().fallback_tool_count(),
        },
    ))
}
