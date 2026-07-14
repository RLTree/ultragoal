use super::*;

pub(crate) fn prepare_routine_execution(
    context: &LiveContext,
    graph: &ImpactGraph,
    snapshot: &DirtySnapshot,
    plan: &RoutinePlan,
    spec: RoutineAdapterSpec,
) -> Result<PreparedRoutineExecution, RoutineError> {
    validate_result_scope(&spec.result_scope)?;
    let binding = structural_binding(context, plan, "adapter-plan-binding-is-stale")?;
    snapshot.require_complete_capture()?;
    if snapshot.binding() != &binding
        || plan.graph_id() != graph.graph_id()
        || plan.snapshot_id() != snapshot.snapshot_id()
    {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            "adapter-context-graph-snapshot-plan-mismatch",
            None,
        ));
    }
    if plan.checks().len() > MAX_SELECTED_CHECKS {
        return Err(adapter_error("adapter-selected-check-limit-exceeded"));
    }
    if plan.checks().is_empty() {
        if plan.affected_set().mode() != PlanMode::NoOp || !spec.invocations.is_empty() {
            return Err(adapter_error("adapter-noop-invocation-set-not-empty"));
        }
        let selected = Vec::<String>::new();
        let support_limit = "non-effectful protocol projection only; no public command or claim";
        let projection_id = digest_of(&NoOpPayload {
            binding_id: binding.binding_id(),
            graph_id: graph.graph_id(),
            snapshot_id: snapshot.snapshot_id(),
            plan_id: plan.plan_id(),
            result_scope: &spec.result_scope,
            selected: &selected,
            effect_intent_count: 0,
            support_limit,
        })?;
        return Ok(PreparedRoutineExecution::NoOp(RoutineNoOpProjection::new(
            projection_id,
            &binding,
            graph.graph_id().to_owned(),
            snapshot.snapshot_id().to_owned(),
            plan.plan_id().to_owned(),
            spec.result_scope,
        )));
    }
    if plan.affected_set().mode() == PlanMode::NoOp {
        return Err(adapter_error("adapter-nonempty-noop-plan-invalid"));
    }
    validate_invocation_set(plan, &spec.invocations)?;

    let working_directory = binding
        .worktree_root()
        .to_str()
        .ok_or_else(|| adapter_error("adapter-working-directory-not-utf8"))?
        .to_owned();
    let mut bound = Vec::with_capacity(plan.checks().len());
    for (plan_order, (check, invocation)) in plan
        .checks()
        .iter()
        .zip(spec.invocations.into_iter())
        .enumerate()
    {
        let runner = exact_runner(context, check)?;
        mediator::validate_read_sources(binding.worktree_root(), &invocation.read_sources)?;
        validate_bound_invocation(&invocation, &runner, check)?;
        let invocation = validate_and_normalize_bound_invocation(invocation)?;
        let mut argv = Vec::with_capacity(invocation.arguments.len() + 1);
        argv.push(invocation.tool_name.clone());
        argv.extend(invocation.arguments);
        bound.push(BoundIntent {
            plan_order,
            node_id: check.node_id().to_owned(),
            selected_tool: invocation.tool_name,
            tool_identity_sha256: invocation.tool_identity_sha256,
            program_path_hex: invocation.program_path_hex,
            program_sha256: invocation.program_sha256,
            program_byte_length: invocation.program_byte_length,
            program_unix_mode: invocation.program_unix_mode,
            argv,
            working_directory: working_directory.clone(),
            environment_policy: "clear-all-allowlisted-v1",
            environment_keys: invocation.environment.keys().cloned().collect(),
            environment_sha256: invocation.environment_sha256.clone(),
            environment: invocation.environment,
            read_authority_policy: "default-deny-exact-bound-read-v1",
            read_source_paths: invocation
                .read_sources
                .iter()
                .map(|source| source.relative_path.clone())
                .collect(),
            read_authority_sha256: invocation.read_authority_sha256.clone(),
            read_sources: invocation.read_sources,
            mediation_preflight:
                "revalidate-context-candidate-tool-executable-read-sources-output-scopes-before-and-after-effect-v1",
            timeout_ms: invocation.timeout_ms,
            output_budget_bytes: invocation.output_budget_bytes,
            declared_output_scopes: invocation.declared_output_scopes,
            expected_dependency_nodes: check.depends_on().iter().cloned().collect(),
            input_id: check.input_id().to_owned(),
        });
    }
    let intent_ids = bound.iter().map(digest_of).collect::<Result<Vec<_>, _>>()?;
    let commitments = intent_ids
        .iter()
        .zip(bound.iter())
        .map(|(intent_id, intent)| IntentCommitment { intent_id, intent })
        .collect::<Vec<_>>();
    let protocol_id = digest_of(&ProtocolPayload {
        binding_id: binding.binding_id(),
        graph_id: graph.graph_id(),
        snapshot_id: snapshot.snapshot_id(),
        plan_id: plan.plan_id(),
        result_scope: &spec.result_scope,
        intents: &commitments,
    })?;
    let intents = bound
        .into_iter()
        .zip(intent_ids)
        .map(|(intent, intent_id)| {
            RoutineEffectIntent::new(
                protocol_id.clone(),
                intent_id,
                intent.plan_order,
                intent.node_id,
                intent.selected_tool,
                intent.tool_identity_sha256,
                intent.program_path_hex,
                intent.program_sha256,
                intent.program_byte_length,
                intent.program_unix_mode,
                intent.argv,
                intent.working_directory,
                intent.environment_sha256,
                intent.environment,
                intent.read_authority_sha256,
                intent.read_sources,
                intent.timeout_ms,
                intent.output_budget_bytes,
                intent.declared_output_scopes,
                intent.expected_dependency_nodes,
                intent.input_id,
            )
        })
        .collect::<Vec<_>>();
    let issuance = NEXT_REQUEST_ISSUANCE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| adapter_error("adapter-request-issuance-exhausted"))?;
    let issuance_bytes = issuance.to_be_bytes();
    let request_id = framed(&[
        REQUEST_DOMAIN,
        protocol_id.as_bytes(),
        binding.context_id().as_bytes(),
        &issuance_bytes,
    ]);
    let seal_id = request_seal(&request_id, &protocol_id, issuance);
    Ok(PreparedRoutineExecution::Effect(RoutineEffectRequest::new(
        request_id,
        protocol_id,
        binding,
        graph.graph_id().to_owned(),
        snapshot.snapshot_id().to_owned(),
        plan.plan_id().to_owned(),
        spec.result_scope,
        intents,
        issuance,
        seal_id,
    )))
}
