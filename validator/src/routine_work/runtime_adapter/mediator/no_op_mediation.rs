use super::*;

pub(crate) fn mediate_noop(
    context: &LiveContext,
    plan: &RoutinePlan,
    projection: RoutineNoOpProjection,
    reuse: RoutineReuseInput,
) -> Result<RoutineMediationResult, RoutineError> {
    if !reuse.into_artifacts().is_empty() {
        return Err(mediator_error("mediator-noop-authority-or-reuse-present"));
    }
    let current = RoutineBinding::from_live(context)?;
    if &current != plan.binding()
        || projection.context_id() != context.context_id()
        || projection.candidate_id() != plan.binding().candidate_id()
        || projection.plan_id() != plan.plan_id()
        || !projection.selected().is_empty()
        || projection.effect_intent_count() != 0
    {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            "mediator-noop-binding-invalid",
            None,
        ));
    }
    Ok(RoutineMediationResult {
        request_id: None,
        protocol_id: None,
        status: RoutineMediatorStatus::CompleteNoOp,
        nodes: Vec::new(),
        recovery_marker: None,
        continuation: None,
        attempt_grant: None,
        checkpoint_head: None,
        support_limit: MEDIATOR_SUPPORT_LIMIT,
    })
}

pub(crate) enum IntentResult {
    Executed(ExecutedArtifact),
    Incomplete {
        disposition: RoutineNodeDisposition,
        failure_code: &'static str,
    },
}
