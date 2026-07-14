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
        (None, None) => {}
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
    })
}

pub(crate) fn preflight_request(
    context: &LiveContext,
    plan: &RoutinePlan,
    request: &RoutineEffectRequest,
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
        let outputs = OutputConfinement::prepare(
            &root,
            intent.declared_output_scopes(),
            intent.output_budget_bytes(),
        )?;
        let reads = ReadConfinement::open_bound(&root, intent.read_sources())?;
        program.validate()?;
        reads.validate(&root)?;
        outputs.validate()?;
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
    if intent.selected_tool() != check.selected_tool()
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
    token: &RoutineMediatedIntent,
) -> Result<BTreeMap<String, String>, RoutineError> {
    let mut environment = token.intent().environment().clone();
    for (key, value) in [
        ("HUL_ROUTINE_REQUEST_ID", token.request_id()),
        ("HUL_ROUTINE_PROTOCOL_ID", token.protocol_id()),
        ("HUL_ROUTINE_INTENT_ID", token.intent().intent_id()),
        ("HUL_ROUTINE_NODE_ID", token.intent().node_id()),
    ] {
        if environment
            .insert(key.to_owned(), value.to_owned())
            .is_some()
        {
            return Err(mediator_error("mediator-environment-reserved-name"));
        }
    }
    Ok(environment)
}

pub(crate) fn parse_command_report(
    bytes: &[u8],
    token: &RoutineMediatedIntent,
) -> Result<CommandReport, RoutineError> {
    let report: CommandReport = serde_json::from_slice(bytes)
        .map_err(|_| mediator_error("mediator-command-report-invalid"))?;
    if canonical(&report)? != bytes
        || report.schema_version != "RoutineCommandReport-v1"
        || report.request_id != token.request_id()
        || report.protocol_id != token.protocol_id()
        || report.intent_id != token.intent().intent_id()
        || report.node_id != token.intent().node_id()
    {
        return Err(mediator_error("mediator-command-report-binding-invalid"));
    }
    Ok(report)
}
