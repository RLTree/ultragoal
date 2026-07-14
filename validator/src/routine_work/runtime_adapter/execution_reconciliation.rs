use super::*;

pub(crate) fn reconcile_routine_execution(
    context: &LiveContext,
    plan: &RoutinePlan,
    authority: RoutineMediationAuthority,
    outcomes: Vec<RoutineMediatedOutcome>,
) -> Result<RoutineReport, RoutineError> {
    let binding = structural_binding(context, plan, "adapter-result-plan-binding-is-stale")?;
    if authority.binding != binding
        || authority.plan_id != plan.plan_id()
        || authority.graph_id != plan.graph_id()
        || authority.snapshot_id != plan.snapshot_id()
        || authority.expected.is_empty()
    {
        return Err(RoutineError::new(
            RoutineErrorId::ContextMismatch,
            "adapter-mediation-authority-binding-invalid",
            None,
        ));
    }
    validate_outcomes(&authority, &outcomes)?;
    authority.require_complete()?;
    let records = authority
        .expected
        .iter()
        .zip(outcomes)
        .map(|(expected, outcome)| ReportRecord::new(&expected.node_id, outcome.disposition))
        .collect::<Vec<_>>();
    let report = reconcile_report(context, plan, &authority.execution_result_scope, records)?;
    if report.status() != ReportStatus::CompleteExecution {
        return Err(adapter_error("adapter-report-incomplete"));
    }
    authority.finish()?;
    Ok(report)
}

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

pub(crate) fn validate_outcomes(
    authority: &RoutineMediationAuthority,
    outcomes: &[RoutineMediatedOutcome],
) -> Result<(), RoutineError> {
    if outcomes.len() != authority.expected.len() {
        return Err(adapter_error("adapter-outcome-cardinality-inexact"));
    }
    let mut seen = BTreeSet::new();
    for (expected, outcome) in authority.expected.iter().zip(outcomes) {
        if !seen.insert(outcome.intent_id.as_str()) {
            return Err(adapter_error("adapter-outcome-duplicated"));
        }
        if !authority.same_issuance(outcome)
            || outcome.request_id != authority.request_id
            || outcome.protocol_id != authority.protocol_id
        {
            return Err(adapter_error("adapter-outcome-session-mismatch"));
        }
        if !authority
            .expected
            .iter()
            .any(|row| row.intent_id == outcome.intent_id)
        {
            return Err(adapter_error("adapter-outcome-unknown"));
        }
        if outcome.plan_order != expected.plan_order
            || outcome.intent_id != expected.intent_id
            || outcome.node_id != expected.node_id
        {
            return Err(adapter_error("adapter-outcome-order-invalid"));
        }
        if !outcome.observed {
            return Err(adapter_error("adapter-outcome-unobserved"));
        }
    }
    for (expected, outcome) in authority.expected.iter().zip(outcomes) {
        match &outcome.disposition {
            ReportDisposition::Executed(work)
                if work.node_id() == expected.node_id
                    && work.result_scope() == authority.execution_result_scope
                    && work.outcome() == RunOutcome::Passed
                    && work.behavior_observed() => {}
            ReportDisposition::Reused(evidence)
                if evidence.node_id() == expected.node_id
                    && evidence.result_scope() == authority.execution_result_scope => {}
            ReportDisposition::Executed(_) | ReportDisposition::Reused(_) => {
                return Err(adapter_error("adapter-outcome-witness-binding-invalid"));
            }
            ReportDisposition::Skipped(_) | ReportDisposition::Failed { .. } => {
                return Err(adapter_error("adapter-outcome-not-complete"));
            }
        }
    }
    Ok(())
}

pub(crate) fn mediation_result_scope(request_id: &str) -> Result<String, RoutineError> {
    let digest = request_id
        .strip_prefix("sha256:")
        .filter(|value| value.len() == 64)
        .ok_or_else(|| adapter_error("adapter-request-id-invalid"))?;
    let scope = format!("rma:{digest}");
    validate_result_scope(&scope)?;
    Ok(scope)
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
