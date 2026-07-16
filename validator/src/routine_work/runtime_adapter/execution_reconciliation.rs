use super::*;

pub(crate) fn validate_invocation_set(
    plan: &RoutinePlan,
    invocations: &[RoutineInvocationSpec],
) -> Result<(), RoutineError> {
    if invocations.len() != plan.checks().len() {
        return Err(adapter_error("adapter-invocation-cardinality-inexact"));
    }
    if execution_authority::duplicate_node(invocations).is_some() {
        return Err(adapter_error("adapter-invocation-node-duplicated"));
    }
    let selected = plan
        .checks()
        .iter()
        .map(PlannedCheck::node_id)
        .collect::<BTreeSet<_>>();
    if invocations
        .iter()
        .any(|invocation| !selected.contains(invocation.node_id()))
    {
        return Err(adapter_error("adapter-invocation-node-unknown"));
    }
    if plan
        .checks()
        .iter()
        .zip(invocations)
        .any(|(check, invocation)| check.node_id() != invocation.node_id())
    {
        return Err(adapter_error("adapter-invocation-order-invalid"));
    }
    Ok(())
}

pub(crate) fn structural_binding(
    context: &LiveContext,
    plan: &RoutinePlan,
    cause: &'static str,
) -> Result<RoutineBinding, RoutineError> {
    ensure_read_only_context(context)?;
    let binding = RoutineBinding::from_live(context)?;
    if &binding != plan.binding() {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            cause,
            None,
        ));
    }
    Ok(binding)
}

pub(crate) fn ensure_read_only_context(context: &LiveContext) -> Result<(), RoutineError> {
    if context.effect().authorize(EffectClass::Read).is_err()
        || [
            EffectClass::PlannedWrite,
            EffectClass::WorkspaceWrite,
            EffectClass::ExternalWrite,
            EffectClass::Destructive,
        ]
        .into_iter()
        .any(|effect| context.effect().authorize(effect).is_ok())
    {
        return Err(adapter_error("adapter-read-only-context-required"));
    }
    Ok(())
}

pub(crate) fn exact_runner(
    context: &LiveContext,
    check: &PlannedCheck,
) -> Result<RunnerIdentity, RoutineError> {
    if check.selected_tool() != "ultragoal" {
        return Err(adapter_error("adapter-closed-runner-required"));
    }
    let tool = context
        .capabilities()
        .tool(check.selected_tool())
        .filter(|tool| tool.available)
        .ok_or_else(|| {
            RoutineError::new(
                RoutineErrorId::CapabilityUnavailable,
                "adapter-selected-runner-unavailable",
                None,
            )
        })?;
    let tool_identity_sha256 = digest_of(tool)?;
    if tool_identity_sha256 != check.selected_tool_identity() {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            "adapter-selected-runner-identity-stale",
            None,
        ));
    }
    let runner = runner_identity(tool, tool_identity_sha256)?;
    let expected = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(|_| adapter_error("adapter-current-runner-unavailable"))?;
    if Path::new(&runner.program_path) != expected {
        return Err(adapter_error("adapter-current-runner-substituted"));
    }
    Ok(runner)
}
