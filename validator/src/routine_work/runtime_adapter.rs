//! Internal effect-request protocol for routine execution.
//!
//! This module prepares opaque intents, binds read-only evidence expectations,
//! and reconciles already-observed outcomes. It owns no workspace-effect grant
//! and performs no workspace, receipt, cache, telemetry, network, or external
//! write.

mod model;

use serde::Serialize;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::context::{EffectClass, LiveContext, ToolCapability};

use super::digest::{digest_of, framed, valid};
use super::{
    DependencyResult, DirtySnapshot, ImpactGraph, PlanMode, PlannedCheck, RepoPath,
    ReportDisposition, ReportRecord, ReportStatus, ReuseExpectation, RoutineBinding, RoutineError,
    RoutineErrorId, RoutinePlan, RoutineReport, RunOutcome, reconcile_report,
};

pub(crate) use model::{
    PreparedRoutineExecution, RoutineAdapterSpec, RoutineEffectIntent, RoutineEffectRequest,
    RoutineInvocationSpec, RoutineMediatedExpectation, RoutineMediatedIntent,
    RoutineMediatedOutcome, RoutineMediatedWitness, RoutineMediationAuthority,
    RoutineMediationBatch, RoutineNoOpProjection,
};

const MAX_SELECTED_CHECKS: usize = 4_096;
const MAX_ARGUMENTS: usize = 128;
const MAX_ARGUMENT_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_SCOPES: usize = 128;
const MAX_TIMEOUT_MS: u64 = 3_600_000;
const MAX_OUTPUT_BUDGET_BYTES: u64 = 64 * 1024 * 1024;
const REQUEST_DOMAIN: &[u8] = b"routine-effect-request-v1";
const REQUEST_SEAL_DOMAIN: &[u8] = b"routine-effect-request-seal-v1";

static NEXT_REQUEST_ISSUANCE: AtomicU64 = AtomicU64::new(1);

#[derive(Serialize)]
struct BoundIntent {
    plan_order: usize,
    node_id: String,
    selected_tool: String,
    tool_identity_sha256: String,
    program_path_hex: String,
    program_sha256: String,
    program_byte_length: u64,
    program_unix_mode: Option<u32>,
    argv: Vec<String>,
    working_directory: String,
    environment_policy: &'static str,
    mediation_preflight: &'static str,
    timeout_ms: u64,
    output_budget_bytes: u64,
    declared_output_scopes: Vec<RepoPath>,
    expected_dependency_nodes: Vec<String>,
    input_id: String,
}

#[derive(Serialize)]
struct IntentCommitment<'a> {
    intent_id: &'a str,
    intent: &'a BoundIntent,
}

#[derive(Serialize)]
struct ProtocolPayload<'a> {
    binding_id: &'a str,
    graph_id: &'a str,
    snapshot_id: &'a str,
    plan_id: &'a str,
    result_scope: &'a str,
    intents: &'a [IntentCommitment<'a>],
}

#[derive(Serialize)]
struct NoOpPayload<'a> {
    binding_id: &'a str,
    graph_id: &'a str,
    snapshot_id: &'a str,
    plan_id: &'a str,
    result_scope: &'a str,
    selected: &'a [String],
    effect_intent_count: usize,
    support_limit: &'static str,
}

struct RunnerIdentity {
    tool_name: String,
    tool_identity_sha256: String,
    program_path_hex: String,
    program_sha256: String,
    program_byte_length: u64,
    program_unix_mode: Option<u32>,
}

pub(crate) fn bind_routine_invocation(
    context: &LiveContext,
    plan: &RoutinePlan,
    node_id: &str,
    arguments: Vec<String>,
    timeout_ms: u64,
    output_budget_bytes: u64,
    declared_output_scopes: Vec<RepoPath>,
) -> Result<RoutineInvocationSpec, RoutineError> {
    let binding = structural_binding(context, plan, "adapter-invocation-plan-binding-is-stale")?;
    let check = plan
        .check(node_id)
        .ok_or_else(|| adapter_error("adapter-invocation-node-unknown"))?;
    let runner = exact_runner(context, check)?;
    let declared_output_scopes = normalized_output_scopes(declared_output_scopes)?;
    validate_execution_policy(
        &arguments,
        timeout_ms,
        output_budget_bytes,
        &declared_output_scopes,
    )?;
    if binding.worktree_root().to_str().is_none() {
        return Err(adapter_error("adapter-working-directory-not-utf8"));
    }
    Ok(RoutineInvocationSpec::bound(
        node_id.to_owned(),
        runner.tool_name,
        runner.tool_identity_sha256,
        runner.program_path_hex,
        runner.program_sha256,
        runner.program_byte_length,
        runner.program_unix_mode,
        arguments,
        timeout_ms,
        output_budget_bytes,
        declared_output_scopes,
    ))
}

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
            environment_policy: "clear-all-no-inheritance-v1",
            mediation_preflight:
                "revalidate-context-candidate-tool-executable-output-scopes-before-effect-v1",
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
    let expected = model::mediation_rows(&request.intents);
    let seal = model::authority_seal(&request);
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
    let tokens = model::mediation_tokens(
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
        || token.intent.environment_policy() != "clear-all-no-inheritance-v1"
        || token.intent.mediation_preflight()
            != "revalidate-context-candidate-tool-executable-output-scopes-before-effect-v1"
    {
        return Err(adapter_error("adapter-mediated-intent-binding-invalid"));
    }
    validate_execution_policy(
        arguments,
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

fn validate_invocation_set(
    plan: &RoutinePlan,
    invocations: &[RoutineInvocationSpec],
) -> Result<(), RoutineError> {
    if invocations.len() != plan.checks().len() {
        return Err(adapter_error("adapter-invocation-cardinality-inexact"));
    }
    if model::duplicate_node(invocations).is_some() {
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

fn validate_outcomes(
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

fn mediation_result_scope(request_id: &str) -> Result<String, RoutineError> {
    let digest = request_id
        .strip_prefix("sha256:")
        .filter(|value| value.len() == 64)
        .ok_or_else(|| adapter_error("adapter-request-id-invalid"))?;
    let scope = format!("rma:{digest}");
    validate_result_scope(&scope)?;
    Ok(scope)
}

fn structural_binding(
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

fn ensure_read_only_context(context: &LiveContext) -> Result<(), RoutineError> {
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

fn exact_runner(
    context: &LiveContext,
    check: &PlannedCheck,
) -> Result<RunnerIdentity, RoutineError> {
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
    runner_identity(tool, tool_identity_sha256)
}

fn runner_identity(
    tool: &ToolCapability,
    tool_identity_sha256: String,
) -> Result<RunnerIdentity, RoutineError> {
    let executable = tool
        .executable
        .as_deref()
        .ok_or_else(|| adapter_error("adapter-runner-program-path-missing"))?;
    let executable_sha256 = tool
        .executable_sha256
        .as_deref()
        .ok_or_else(|| adapter_error("adapter-runner-program-identity-missing"))?;
    let program_sha256 = format!("sha256:{executable_sha256}");
    let program_byte_length = tool
        .byte_length
        .filter(|length| *length > 0)
        .ok_or_else(|| adapter_error("adapter-runner-program-length-invalid"))?;
    if !valid(&tool_identity_sha256) || !valid(&program_sha256) {
        return Err(adapter_error("adapter-runner-program-identity-invalid"));
    }
    Ok(RunnerIdentity {
        tool_name: tool.name.clone(),
        tool_identity_sha256,
        program_path_hex: hex(executable.as_bytes()),
        program_sha256,
        program_byte_length,
        program_unix_mode: tool.unix_mode,
    })
}

fn validate_bound_invocation(
    invocation: &RoutineInvocationSpec,
    runner: &RunnerIdentity,
    check: &PlannedCheck,
) -> Result<(), RoutineError> {
    if invocation.node_id != check.node_id()
        || invocation.tool_name != runner.tool_name
        || invocation.tool_identity_sha256 != runner.tool_identity_sha256
        || invocation.program_path_hex != runner.program_path_hex
        || invocation.program_sha256 != runner.program_sha256
        || invocation.program_byte_length != runner.program_byte_length
        || invocation.program_unix_mode != runner.program_unix_mode
    {
        return Err(adapter_error("adapter-runner-binding-mismatch"));
    }
    Ok(())
}

fn validate_and_normalize_bound_invocation(
    mut invocation: RoutineInvocationSpec,
) -> Result<RoutineInvocationSpec, RoutineError> {
    invocation.declared_output_scopes =
        normalized_output_scopes(invocation.declared_output_scopes)?;
    validate_execution_policy(
        &invocation.arguments,
        invocation.timeout_ms,
        invocation.output_budget_bytes,
        &invocation.declared_output_scopes,
    )?;
    Ok(invocation)
}

fn validate_execution_policy(
    arguments: &[String],
    timeout_ms: u64,
    output_budget_bytes: u64,
    output_scopes: &[RepoPath],
) -> Result<(), RoutineError> {
    let total_argument_bytes = arguments.iter().map(String::len).sum::<usize>();
    if arguments.len() > MAX_ARGUMENTS
        || total_argument_bytes > MAX_ARGUMENT_BYTES
        || arguments.iter().any(|argument| {
            argument.is_empty()
                || argument.len() > 4_096
                || argument.bytes().any(|byte| byte.is_ascii_control())
        })
    {
        return Err(adapter_error("adapter-argv-invalid"));
    }
    if timeout_ms == 0 || timeout_ms > MAX_TIMEOUT_MS {
        return Err(adapter_error("adapter-timeout-invalid"));
    }
    if output_budget_bytes == 0 || output_budget_bytes > MAX_OUTPUT_BUDGET_BYTES {
        return Err(adapter_error("adapter-output-budget-invalid"));
    }
    if output_scopes.len() > MAX_OUTPUT_SCOPES {
        return Err(adapter_error("adapter-output-scope-limit-exceeded"));
    }
    Ok(())
}

fn normalized_output_scopes(mut scopes: Vec<RepoPath>) -> Result<Vec<RepoPath>, RoutineError> {
    scopes.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    let mut case_keys = BTreeSet::new();
    if scopes
        .iter()
        .any(|scope| !case_keys.insert(scope.case_key()))
    {
        return Err(adapter_error("adapter-output-scope-duplicated"));
    }
    Ok(scopes)
}

fn validate_result_scope(value: &str) -> Result<(), RoutineError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(adapter_error("adapter-result-scope-invalid"));
    }
    Ok(())
}

fn request_seal(request_id: &str, protocol_id: &str, issuance: u64) -> String {
    framed(&[
        REQUEST_SEAL_DOMAIN,
        request_id.as_bytes(),
        protocol_id.as_bytes(),
        &issuance.to_be_bytes(),
    ])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn adapter_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}
