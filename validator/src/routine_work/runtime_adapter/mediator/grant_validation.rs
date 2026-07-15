use super::super::{
    RUST_SOURCE_SYNTAX_ARGUMENTS, RUST_SOURCE_SYNTAX_BEHAVIOR, default_environment, exact_runner,
};
use super::*;

pub(crate) fn validate_grant(
    context: &LiveContext,
    plan: &RoutinePlan,
    request: &RoutineEffectRequest,
    grant: &RoutineRootGrant,
) -> Result<(), RoutineError> {
    let mut expected_scopes = request
        .intents
        .iter()
        .flat_map(|intent| intent.declared_output_scopes().iter().cloned())
        .collect::<Vec<_>>();
    expected_scopes.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    expected_scopes.dedup();
    if grant.grant_id != grant_identity(grant)?
        || grant.seal != grant_seal(grant)?
        || grant.session_id.is_empty()
        || grant.session_id.len() > 128
        || grant.request_id != request.request_id
        || grant.protocol_id != request.protocol_id
        || grant.context_id != context.context_id()
        || grant.context_id != request.binding.context_id()
        || grant.candidate_id != request.binding.candidate_id()
        || grant.plan_id != plan.plan_id()
        || grant.plan_id != request.plan_id
        || grant.snapshot_id != request.snapshot_id
        || grant.allowed_output_scopes != expected_scopes
    {
        return Err(mediator_error("mediator-root-grant-binding-invalid"));
    }
    Ok(())
}

pub(crate) fn reserve_grant(grant: &RoutineRootGrant) -> Result<AttemptReservation, RoutineError> {
    if let Some(durable) = &grant.durable {
        durable.validate_reserved()?;
    }
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.consumed_grants.contains(&grant.grant_id) {
        return Err(mediator_error("mediator-root-grant-replayed"));
    }
    if state.active_protocols.contains_key(&grant.protocol_id) {
        return Err(mediator_error("mediator-protocol-attempt-active"));
    }
    match (
        state.ambiguous_protocols.get(&grant.protocol_id),
        grant.recovery_for.as_ref(),
    ) {
        (Some(expected), Some(actual)) if expected == actual => {}
        (Some(_), _) => return Err(mediator_error("mediator-recovery-authority-required")),
        (None, Some(_))
            if grant
                .durable
                .as_ref()
                .is_some_and(|durable| durable.recovery_is_durable()) => {}
        (None, Some(_)) => return Err(mediator_error("mediator-recovery-marker-stale")),
        (None, None) => state
            .failure_records
            .retain(|_, record| record.protocol_id != grant.protocol_id),
    }
    state.consumed_grants.insert(grant.grant_id.clone());
    state
        .active_protocols
        .insert(grant.protocol_id.clone(), grant.grant_id.clone());
    Ok(AttemptReservation {
        protocol_id: grant.protocol_id.clone(),
        grant_id: grant.grant_id.clone(),
        recovery_marker: recovery_identity(&grant.grant_id, &grant.protocol_id, &grant.request_id),
        prior_recovery_marker: grant.recovery_for.clone(),
        started: Cell::new(false),
        settled: Cell::new(false),
        durable: grant.durable.clone(),
        staged: RefCell::new(Vec::new()),
    })
}

pub(crate) fn preflight_request(
    context: &LiveContext,
    plan: &RoutinePlan,
    request: &RoutineEffectRequest,
) -> Result<(), RoutineError> {
    preflight_request_inner(context, plan, request, true)
}

pub(crate) fn preflight_request_without_outputs(
    context: &LiveContext,
    plan: &RoutinePlan,
    request: &RoutineEffectRequest,
) -> Result<(), RoutineError> {
    preflight_request_inner(context, plan, request, false)
}

fn preflight_request_inner(
    context: &LiveContext,
    plan: &RoutinePlan,
    request: &RoutineEffectRequest,
    require_outputs: bool,
) -> Result<(), RoutineError> {
    context
        .revalidate()
        .map_err(|_| concurrent("mediator-context-preflight-stale"))?;
    let current = RoutineBinding::from_live(context)?;
    if &current != plan.binding()
        || request.binding != current
        || request.plan_id != plan.plan_id()
        || request.snapshot_id != plan.snapshot_id()
        || request.graph_id != plan.graph_id()
    {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            "mediator-request-context-plan-stale",
            None,
        ));
    }
    for intent in &request.intents {
        validate_intent(context, plan, intent)?;
        let root = RootAnchor::open(context.worktree_root())?;
        let program = PinnedExecutable::open_bound(
            intent.program_path_hex(),
            intent.program_sha256(),
            intent.program_byte_length(),
            intent.program_unix_mode(),
        )?;
        let outputs = require_outputs
            .then(|| {
                OutputConfinement::prepare(
                    &root,
                    intent.declared_output_scopes(),
                    intent.output_budget_bytes(),
                )
            })
            .transpose()?;
        let reads = ReadConfinement::open_bound(&root, intent.read_sources())?;
        program.validate()?;
        reads.validate(&root)?;
        if let Some(outputs) = outputs {
            outputs.validate()?;
        }
        root.validate()?;
    }
    validate_snapshot(context, &request.snapshot_id)?;
    Ok(())
}

pub(crate) fn validate_snapshot(context: &LiveContext, expected: &str) -> Result<(), RoutineError> {
    let current = LocalDirtyTree::capture(context)
        .map_err(|_| concurrent("mediator-dirty-snapshot-recapture-failed"))?;
    if current.snapshot_id() != expected {
        return Err(concurrent("mediator-dirty-snapshot-stale"));
    }
    Ok(())
}

pub(crate) fn validate_intent(
    context: &LiveContext,
    plan: &RoutinePlan,
    intent: &RoutineEffectIntent,
) -> Result<(), RoutineError> {
    let check = plan
        .checks()
        .get(intent.plan_order())
        .filter(|check| check.node_id() == intent.node_id())
        .ok_or_else(|| mediator_error("mediator-intent-plan-order-invalid"))?;
    let tool = context
        .capabilities()
        .tool(check.selected_tool())
        .filter(|tool| tool.available)
        .ok_or_else(|| mediator_error("mediator-runner-unavailable"))?;
    let expected_environment = default_environment(&exact_runner(context, check)?)?;
    let expected_argv = std::iter::once("ultragoal")
        .chain(RUST_SOURCE_SYNTAX_ARGUMENTS)
        .collect::<Vec<_>>();
    if intent.behavior_id() != RUST_SOURCE_SYNTAX_BEHAVIOR
        || intent.selected_tool() != "ultragoal"
        || intent.argv() != expected_argv
        || intent.environment() != &expected_environment
        || intent.read_sources().is_empty()
        || intent.selected_tool() != check.selected_tool()
        || intent.tool_identity_sha256() != check.selected_tool_identity()
        || digest_of(tool)? != intent.tool_identity_sha256()
        || intent.input_id() != check.input_id()
        || intent.expected_dependency_nodes()
            != check.depends_on().iter().cloned().collect::<Vec<_>>()
        || intent.argv().first().map(String::as_str) != Some(intent.selected_tool())
        || intent.working_directory() != context.worktree_root().to_string_lossy()
        || intent.environment_policy() != "clear-all-allowlisted-v1"
        || environment_digest(intent.environment())? != intent.environment_sha256()
        || intent.read_authority_policy() != "default-deny-exact-bound-read-v1"
        || intent.read_source_paths()
            != intent
                .read_sources()
                .iter()
                .map(|source| source.relative_path.clone())
                .collect::<Vec<_>>()
        || read_authority_digest(intent.read_sources())? != intent.read_authority_sha256()
    {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            "mediator-intent-binding-stale",
            None,
        ));
    }
    Ok(())
}

pub(crate) fn execution_environment(
    _token: &RoutineMediatedIntent,
) -> Result<BTreeMap<String, String>, RoutineError> {
    Ok(_token.intent().environment().clone())
}
