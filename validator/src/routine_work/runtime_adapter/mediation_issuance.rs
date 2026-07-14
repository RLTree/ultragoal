use super::*;

pub(crate) fn begin_routine_mediation(
    context: &LiveContext,
    plan: &RoutinePlan,
    request: RoutineEffectRequest,
) -> Result<RoutineMediationBatch, RoutineError> {
    let binding = structural_binding(context, plan, "adapter-mediation-plan-binding-is-stale")?;
    if request.binding != binding
        || request.plan_id != plan.plan_id()
        || request.graph_id != plan.graph_id()
        || request.snapshot_id != plan.snapshot_id()
        || request.intents.is_empty()
        || !request.seal_matches(&request_seal(
            &request.request_id,
            &request.protocol_id,
            request.seal_issuance(),
        ))
    {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            "adapter-effect-request-binding-invalid",
            None,
        ));
    }
    request.begin_mediation()?;
    let execution_result_scope = mediation_result_scope(&request.request_id)?;
    let expected = execution_authority::mediation_rows(&request.intents);
    let seal = execution_authority::authority_seal(&request);
    let RoutineEffectRequest {
        request_id,
        protocol_id,
        binding,
        graph_id,
        snapshot_id,
        plan_id,
        result_scope,
        intents,
        seal: _,
    } = request;
    let tokens = execution_authority::mediation_tokens(
        &request_id,
        &protocol_id,
        &execution_result_scope,
        intents,
        &seal,
    );
    let authority = RoutineMediationAuthority::new(
        request_id,
        protocol_id,
        binding,
        graph_id,
        snapshot_id,
        plan_id,
        result_scope,
        execution_result_scope,
        expected,
        seal,
    );
    Ok(RoutineMediationBatch::new(authority, tokens))
}

pub(crate) fn bind_mediated_expectation(
    context: &LiveContext,
    plan: &RoutinePlan,
    token: &RoutineMediatedIntent,
    dependencies: Vec<DependencyResult>,
) -> Result<RoutineMediatedExpectation, RoutineError> {
    token.require_current()?;
    structural_binding(context, plan, "adapter-mediation-plan-binding-is-stale")?;
    let check = plan
        .checks()
        .get(token.intent.plan_order())
        .filter(|check| check.node_id() == token.intent.node_id())
        .ok_or_else(|| adapter_error("adapter-mediated-intent-plan-mismatch"))?;
    let runner = exact_runner(context, check)?;
    let (program, arguments) = token
        .intent
        .argv()
        .split_first()
        .ok_or_else(|| adapter_error("adapter-mediated-intent-argv-invalid"))?;
    if token.intent.protocol_id() != token.protocol_id
        || token.intent.selected_tool() != check.selected_tool()
        || program != token.intent.selected_tool()
        || token.intent.tool_identity_sha256() != check.selected_tool_identity()
        || token.intent.tool_identity_sha256() != runner.tool_identity_sha256
        || token.intent.program_path_hex() != runner.program_path_hex
        || token.intent.program_sha256() != runner.program_sha256
        || token.intent.program_byte_length() != runner.program_byte_length
        || token.intent.program_unix_mode() != runner.program_unix_mode
        || token.intent.input_id() != check.input_id()
        || token.intent.expected_dependency_nodes()
            != check.depends_on().iter().cloned().collect::<Vec<_>>()
        || Some(token.intent.working_directory()) != plan.binding().worktree_root().to_str()
        || token.intent.environment_policy() != "clear-all-allowlisted-v1"
        || token.intent.environment_keys()
            != token
                .intent
                .environment()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        || environment_digest(token.intent.environment())? != token.intent.environment_sha256()
        || token.intent.read_authority_policy() != "default-deny-exact-bound-read-v1"
        || token.intent.read_source_paths()
            != token
                .intent
                .read_sources()
                .iter()
                .map(|source| source.relative_path.clone())
                .collect::<Vec<_>>()
        || read_authority_digest(token.intent.read_sources())?
            != token.intent.read_authority_sha256()
        || token.intent.mediation_preflight()
            != "revalidate-context-candidate-tool-executable-read-sources-output-scopes-before-and-after-effect-v1"
    {
        return Err(adapter_error("adapter-mediated-intent-binding-invalid"));
    }
    validate_execution_policy(
        arguments,
        token.intent.environment(),
        token.intent.timeout_ms(),
        token.intent.output_budget_bytes(),
        token.intent.declared_output_scopes(),
    )?;
    let expectation = ReuseExpectation::for_check(
        context,
        plan,
        check,
        dependencies,
        token.execution_result_scope.clone(),
    )?;
    token.require_current()?;
    Ok(RoutineMediatedExpectation::new(token, expectation))
}

pub(crate) fn bind_mediated_witness(
    expectation: RoutineMediatedExpectation,
    disposition: ReportDisposition,
) -> Result<RoutineMediatedWitness, RoutineError> {
    expectation.require_current()?;
    match &disposition {
        ReportDisposition::Executed(work)
            if work.node_id() == expectation.node_id()
                && work.result_scope() == expectation.execution_result_scope()
                && work.outcome() == RunOutcome::Passed
                && work.behavior_observed() => {}
        ReportDisposition::Reused(evidence)
            if evidence.node_id() == expectation.node_id()
                && evidence.result_scope() == expectation.execution_result_scope() => {}
        ReportDisposition::Executed(_) | ReportDisposition::Reused(_) => {
            return Err(adapter_error("adapter-outcome-witness-binding-invalid"));
        }
        ReportDisposition::Skipped(_) | ReportDisposition::Failed { .. } => {
            return Err(adapter_error("adapter-mediated-witness-not-complete"));
        }
    }
    Ok(expectation.into_witness(disposition))
}

pub(crate) fn observe_mediated_outcome(
    token: RoutineMediatedIntent,
    witness: RoutineMediatedWitness,
) -> Result<RoutineMediatedOutcome, RoutineError> {
    token.require_current()?;
    if !token.same_issuance_witness(&witness) {
        return Err(adapter_error("adapter-outcome-witness-issuance-mismatch"));
    }
    let disposition = witness.disposition;
    token.advance()?;
    Ok(RoutineMediatedOutcome::observed(token, disposition))
}

pub(crate) fn observe_mediated_incomplete(
    token: RoutineMediatedIntent,
    disposition: ReportDisposition,
) -> Result<RoutineMediatedOutcome, RoutineError> {
    token.require_current()?;
    if !matches!(
        disposition,
        ReportDisposition::Skipped(_) | ReportDisposition::Failed { .. }
    ) {
        return Err(adapter_error("adapter-mediated-incomplete-witness-invalid"));
    }
    token.advance()?;
    Ok(RoutineMediatedOutcome::observed(token, disposition))
}
