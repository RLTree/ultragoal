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
    let expected = execution_authority::mediation_rows(&request.intents);
    let seal = execution_authority::authority_seal(&request);
    let RoutineEffectRequest {
        request_id,
        protocol_id,
        binding,
        graph_id: _,
        snapshot_id,
        plan_id,
        result_scope: _,
        intents,
        seal: _,
    } = request;
    let tokens = execution_authority::mediation_tokens(&request_id, &protocol_id, intents, &seal);
    let authority = RoutineMediationAuthority::new(binding, snapshot_id, plan_id, expected, seal);
    Ok(RoutineMediationBatch::new(authority, tokens))
}
