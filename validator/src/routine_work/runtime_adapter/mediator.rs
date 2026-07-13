//! Root-grant-consuming routine process mediation.
//!
//! This module verifies but never issues production root authority. It keeps
//! public dispatch, cache persistence, and accepted reuse-witness construction
//! outside this internal execution boundary.

mod filesystem;
mod model;
mod process;

use serde::Serialize;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use crate::context::LiveContext;

use super::model::{
    PreparedRoutineExecution, RoutineEffectIntent, RoutineEffectRequest, RoutineMediatedIntent,
    RoutineMediationAuthority, RoutineNoOpProjection, RoutineReadSource,
};
use super::{begin_routine_mediation, environment_digest, read_authority_digest};
use crate::routine_work::digest::{canonical, digest_of, framed, sha256, valid};
use crate::routine_work::{
    LocalDirtyTree, RepoPath, RoutineBinding, RoutineError, RoutineErrorId, RoutinePlan,
};

use filesystem::{OutputConfinement, PinnedExecutable, ReadConfinement, RootAnchor};
use model::{
    CommandReport, ExecutedArtifact, ResultArtifactWire, ReuseArtifactWire, VerifiedReuseArtifact,
};
pub(crate) use model::{
    RoutineCancellation, RoutineMediationResult, RoutineMediatorStatus, RoutineNodeDisposition,
    RoutineNodeMediation, RoutineReuseInput, RoutineRootGrant,
};
use process::ProcessTermination;

const GRANT_DOMAIN: &[u8] = b"routine-root-grant-v1";
const GRANT_SEAL_DOMAIN: &[u8] = b"routine-root-grant-seal-v1";
const RESULT_DOMAIN: &[u8] = b"routine-mediated-result-v1";
const RECOVERY_DOMAIN: &[u8] = b"routine-mediated-recovery-v1";
const SUPPORT_LIMIT: &str = "internal macOS single-process routine mediation evidence only; grant replay, reuse authentication, ambiguity recovery, and artifacts are process-local; executable paths must be immutable to this user; after sandbox activation only the exact pinned executable identity may execute, unbound file reads are denied, explicitly bound worktree-relative regular-file reads are identity/content/ctime revalidated, immutable system runtime roots remain policy-authorized, post-activation file-backed executable mapping is limited to immutable system-library roots, and process-fork kills the runner; external interpreted sources, startup-loader environments, executable trampolines, different-object aliases, shebang scripts, descriptor aliases, user-owned executable mappings, and multi-process runners are unsupported; canonical root issuance, durable persistence, public dispatch, installed behavior, and claim decisions remain absent";

pub(super) fn bind_read_sources(
    root: &Path,
    sources: &[RepoPath],
) -> Result<Vec<RoutineReadSource>, RoutineError> {
    if sources.is_empty() {
        return Ok(Vec::new());
    }
    let root = RootAnchor::open(root)?;
    ReadConfinement::bind_records(&root, sources)
}

pub(super) fn validate_read_sources(
    root: &Path,
    sources: &[RoutineReadSource],
) -> Result<(), RoutineError> {
    if sources.is_empty() {
        return Ok(());
    }
    let root = RootAnchor::open(root)?;
    ReadConfinement::open_bound(&root, sources)?.validate(&root)
}

#[derive(Default)]
struct MediatorRegistry {
    consumed_grants: BTreeSet<String>,
    authenticated_artifacts: BTreeMap<String, String>,
    ambiguous_protocols: BTreeMap<String, String>,
    active_protocols: BTreeMap<String, String>,
}

fn registry() -> &'static Mutex<MediatorRegistry> {
    static REGISTRY: OnceLock<Mutex<MediatorRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(MediatorRegistry::default()))
}

/// Process-local attempt reservation guarding the gap between grant
/// consumption and final reconciliation.
///
/// A protocol is reserved before mediation begins. The first successful child
/// spawn records the recovery marker before any later process, filesystem, or
/// reconciliation error can return. Dropping an unsettled reservation releases
/// only the active slot; it deliberately retains a marker once a child may have
/// started.
struct AttemptReservation {
    protocol_id: String,
    grant_id: String,
    recovery_marker: String,
    prior_recovery_marker: Option<String>,
    started: Cell<bool>,
    settled: Cell<bool>,
}

impl AttemptReservation {
    fn mark_started(&self) {
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state
            .ambiguous_protocols
            .insert(self.protocol_id.clone(), self.recovery_marker.clone());
        self.started.set(true);
    }

    fn settle_success(&self) {
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        release_active(&mut state, &self.protocol_id, &self.grant_id);
        let expected = if self.started.get() {
            Some(&self.recovery_marker)
        } else {
            self.prior_recovery_marker.as_ref()
        };
        if expected.is_some_and(|expected| {
            state.ambiguous_protocols.get(&self.protocol_id) == Some(expected)
        }) {
            state.ambiguous_protocols.remove(&self.protocol_id);
        }
        self.settled.set(true);
    }

    fn settle_incomplete(&self) -> Option<String> {
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        release_active(&mut state, &self.protocol_id, &self.grant_id);
        self.settled.set(true);
        if self.started.get() {
            Some(self.recovery_marker.clone())
        } else {
            self.prior_recovery_marker.clone()
        }
    }
}

impl Drop for AttemptReservation {
    fn drop(&mut self) {
        if self.settled.get() {
            return;
        }
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        release_active(&mut state, &self.protocol_id, &self.grant_id);
        if self.started.get() {
            state
                .ambiguous_protocols
                .insert(self.protocol_id.clone(), self.recovery_marker.clone());
        }
    }
}

fn release_active(state: &mut MediatorRegistry, protocol_id: &str, grant_id: &str) {
    if state.active_protocols.get(protocol_id).map(String::as_str) == Some(grant_id) {
        state.active_protocols.remove(protocol_id);
    }
}

#[derive(Serialize)]
struct GrantPayload<'a> {
    domain: &'static str,
    session_id: &'a str,
    request_id: &'a str,
    protocol_id: &'a str,
    context_id: &'a str,
    candidate_id: &'a str,
    plan_id: &'a str,
    snapshot_id: &'a str,
    allowed_output_scopes: &'a [RepoPath],
    recovery_for: Option<&'a str>,
}

pub(super) fn grant_identity(grant: &RoutineRootGrant) -> Result<String, RoutineError> {
    digest_of(&GrantPayload {
        domain: "routine-root-grant-v1",
        session_id: &grant.session_id,
        request_id: &grant.request_id,
        protocol_id: &grant.protocol_id,
        context_id: &grant.context_id,
        candidate_id: &grant.candidate_id,
        plan_id: &grant.plan_id,
        snapshot_id: &grant.snapshot_id,
        allowed_output_scopes: &grant.allowed_output_scopes,
        recovery_for: grant.recovery_for.as_deref(),
    })
}

pub(super) fn grant_seal(grant: &RoutineRootGrant) -> Result<String, RoutineError> {
    let payload = canonical(&GrantPayload {
        domain: "routine-root-grant-v1",
        session_id: &grant.session_id,
        request_id: &grant.request_id,
        protocol_id: &grant.protocol_id,
        context_id: &grant.context_id,
        candidate_id: &grant.candidate_id,
        plan_id: &grant.plan_id,
        snapshot_id: &grant.snapshot_id,
        allowed_output_scopes: &grant.allowed_output_scopes,
        recovery_for: grant.recovery_for.as_deref(),
    })?;
    Ok(framed(&[
        GRANT_SEAL_DOMAIN,
        grant.grant_id.as_bytes(),
        &payload,
    ]))
}

pub(crate) fn mediate_prepared_routine_execution(
    context: &LiveContext,
    plan: &RoutinePlan,
    prepared: PreparedRoutineExecution,
    grant: Option<RoutineRootGrant>,
    cancellation: RoutineCancellation,
    reuse: RoutineReuseInput,
) -> Result<RoutineMediationResult, RoutineError> {
    match prepared {
        PreparedRoutineExecution::NoOp(projection) => {
            mediate_noop(context, plan, projection, grant, reuse)
        }
        PreparedRoutineExecution::Effect(request) => {
            mediate_effect(context, plan, request, grant, cancellation, reuse)
        }
    }
}

fn mediate_noop(
    context: &LiveContext,
    plan: &RoutinePlan,
    projection: RoutineNoOpProjection,
    grant: Option<RoutineRootGrant>,
    reuse: RoutineReuseInput,
) -> Result<RoutineMediationResult, RoutineError> {
    if grant.is_some() || !reuse.into_artifacts().is_empty() {
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
        reuse_artifacts: Vec::new(),
        recovery_marker: None,
        support_limit: SUPPORT_LIMIT,
    })
}

fn mediate_effect(
    context: &LiveContext,
    plan: &RoutinePlan,
    request: RoutineEffectRequest,
    grant: Option<RoutineRootGrant>,
    cancellation: RoutineCancellation,
    reuse: RoutineReuseInput,
) -> Result<RoutineMediationResult, RoutineError> {
    let grant = grant.ok_or_else(|| mediator_error("mediator-root-grant-missing"))?;
    validate_grant(context, plan, &request, &grant)?;
    let supplied_reuse = index_reuse_inputs(reuse, &request)?;
    preflight_request(context, plan, &request)?;
    run_test_pre_spawn_hook();
    preflight_request(context, plan, &request)?;
    let request_id = request.request_id().to_owned();
    let protocol_id = request.protocol_id().to_owned();
    let attempt = reserve_grant(&grant)?;
    let snapshot_id = request.snapshot_id.clone();
    let batch = begin_routine_mediation(context, plan, request)?;
    let (authority, intents) = batch.into_parts();
    let mut dependencies = BTreeMap::<String, String>::new();
    let mut nodes = Vec::with_capacity(intents.len());
    let mut artifacts = Vec::<Vec<u8>>::with_capacity(intents.len());
    let mut generated = Vec::<(String, Vec<u8>)>::new();
    let mut incomplete = false;
    let mut cancelled = false;

    for token in intents {
        let expected_dependencies = token
            .intent()
            .expected_dependency_nodes()
            .iter()
            .map(|node| {
                dependencies
                    .get(node)
                    .map(|digest| (node.clone(), digest.clone()))
            })
            .collect::<Option<BTreeMap<_, _>>>();
        if cancelled || cancellation.is_cancelled() {
            cancelled = true;
            incomplete = true;
            token.advance()?;
            nodes.push(incomplete_node(
                &token,
                RoutineNodeDisposition::Cancelled,
                "MEDIATOR-CANCELLED",
            ));
            continue;
        }
        let Some(expected_dependencies) = expected_dependencies else {
            incomplete = true;
            token.advance()?;
            nodes.push(incomplete_node(
                &token,
                RoutineNodeDisposition::DependencyFailed,
                "MEDIATOR-DEPENDENCY-FAILED",
            ));
            continue;
        };
        let result = mediate_intent(
            context,
            plan,
            &token,
            &snapshot_id,
            &expected_dependencies,
            supplied_reuse.get(token.intent().intent_id()),
            &cancellation,
            &attempt,
        );
        match result {
            Ok(IntentResult::Reused(verified)) => {
                let result_sha256 = verified.wire.result_artifact_sha256.clone();
                dependencies.insert(token.intent().node_id().to_owned(), result_sha256.clone());
                nodes.push(success_node(
                    &token,
                    RoutineNodeDisposition::Reused,
                    result_sha256,
                ));
                artifacts.push(verified.canonical_bytes);
                token.advance()?;
            }
            Ok(IntentResult::Executed(executed)) => {
                dependencies.insert(
                    token.intent().node_id().to_owned(),
                    executed.result_sha256.clone(),
                );
                nodes.push(success_node(
                    &token,
                    RoutineNodeDisposition::Executed,
                    executed.result_sha256,
                ));
                let artifact_sha256 = sha256(&executed.reuse_bytes);
                generated.push((artifact_sha256, executed.reuse_bytes.clone()));
                artifacts.push(executed.reuse_bytes);
                token.advance()?;
            }
            Ok(IntentResult::Incomplete {
                disposition,
                failure_code,
                started,
            }) => {
                incomplete = true;
                cancelled |= disposition == RoutineNodeDisposition::Cancelled;
                debug_assert!(!started || attempt.started.get());
                token.advance()?;
                nodes.push(incomplete_node(&token, disposition, failure_code));
            }
            Err(error) => {
                incomplete = true;
                token.advance()?;
                nodes.push(incomplete_node(
                    &token,
                    RoutineNodeDisposition::Failed,
                    error.cause().to_ascii_uppercase().replace('_', "-"),
                ));
            }
        }
    }
    reconcile_internal(context, plan, &authority, &nodes)?;
    run_test_finish_failure_hook(&authority);
    authority.finish()?;
    let recovery_marker = if incomplete {
        artifacts.clear();
        generated.clear();
        attempt.settle_incomplete()
    } else {
        authenticate_generated(generated);
        attempt.settle_success();
        None
    };
    Ok(RoutineMediationResult {
        request_id: Some(request_id),
        protocol_id: Some(protocol_id),
        status: if cancelled {
            RoutineMediatorStatus::Cancelled
        } else if incomplete {
            RoutineMediatorStatus::IncompleteExecution
        } else {
            RoutineMediatorStatus::CompleteExecution
        },
        nodes,
        reuse_artifacts: artifacts,
        recovery_marker,
        support_limit: SUPPORT_LIMIT,
    })
}

enum IntentResult {
    Reused(VerifiedReuseArtifact),
    Executed(ExecutedArtifact),
    Incomplete {
        disposition: RoutineNodeDisposition,
        failure_code: &'static str,
        started: bool,
    },
}

fn mediate_intent(
    context: &LiveContext,
    plan: &RoutinePlan,
    token: &RoutineMediatedIntent,
    snapshot_id: &str,
    dependencies: &BTreeMap<String, String>,
    reuse: Option<&Vec<u8>>,
    cancellation: &RoutineCancellation,
    attempt: &AttemptReservation,
) -> Result<IntentResult, RoutineError> {
    token.require_current()?;
    validate_intent(context, plan, token.intent())?;
    let root = RootAnchor::open(context.worktree_root())?;
    let program = PinnedExecutable::open_bound(
        token.intent().program_path_hex(),
        token.intent().program_sha256(),
        token.intent().program_byte_length(),
        token.intent().program_unix_mode(),
    )?;
    let outputs = OutputConfinement::prepare(
        &root,
        token.intent().declared_output_scopes(),
        token.intent().output_budget_bytes(),
    )?;
    let reads = ReadConfinement::open_bound(&root, token.intent().read_sources())?;
    if let Some(bytes) = reuse
        && let Some(verified) = verify_reuse_artifact(
            bytes,
            context,
            plan,
            token,
            snapshot_id,
            dependencies,
            &outputs,
        )?
    {
        reads.validate(&root)?;
        return Ok(IntentResult::Reused(verified));
    }
    let environment = execution_environment(token)?;
    let observation = process::execute(
        &program,
        &root,
        &outputs,
        &reads,
        token.intent().argv(),
        &environment,
        Duration::from_millis(token.intent().timeout_ms()),
        token.intent().output_budget_bytes(),
        cancellation,
        || attempt.mark_started(),
    );
    let observation = match observation {
        Ok(value) => value,
        Err(error) => {
            return Err(error);
        }
    };
    if observation.started {
        run_test_post_spawn_hook();
    }
    reads.validate(&root)?;
    let disposition = match observation.termination {
        ProcessTermination::Exited(0) => None,
        ProcessTermination::Cancelled => {
            Some((RoutineNodeDisposition::Cancelled, "MEDIATOR-CANCELLED"))
        }
        ProcessTermination::TimedOut => Some((RoutineNodeDisposition::Failed, "MEDIATOR-TIMEOUT")),
        ProcessTermination::OutputLimit => {
            Some((RoutineNodeDisposition::Failed, "MEDIATOR-OUTPUT-LIMIT"))
        }
        ProcessTermination::DescendantSurvived => Some((
            RoutineNodeDisposition::Failed,
            "MEDIATOR-DESCENDANT-SURVIVED",
        )),
        ProcessTermination::CleanupFailed => {
            Some((RoutineNodeDisposition::Failed, "MEDIATOR-CLEANUP-FAILED"))
        }
        ProcessTermination::Exited(_) | ProcessTermination::Signaled(_) => {
            Some((RoutineNodeDisposition::Failed, "MEDIATOR-CHECK-FAILED"))
        }
    };
    if let Some((disposition, failure_code)) = disposition {
        return Ok(IntentResult::Incomplete {
            disposition,
            failure_code,
            started: observation.started,
        });
    }
    let report = parse_command_report(&observation.stdout, token)?;
    let output_files = outputs.capture()?;
    let artifact_bytes = output_files
        .values()
        .try_fold(0_u64, |total, file| total.checked_add(file.byte_length));
    if artifact_bytes
        .and_then(|total| total.checked_add(observation.output_byte_length))
        .is_none_or(|total| total > token.intent().output_budget_bytes())
    {
        return Ok(IntentResult::Incomplete {
            disposition: RoutineNodeDisposition::Failed,
            failure_code: "MEDIATOR-OUTPUT-LIMIT",
            started: true,
        });
    }
    outputs.validate()?;
    reads.validate(&root)?;
    root.validate()?;
    program.validate()?;
    context
        .revalidate()
        .map_err(|_| concurrent("mediator-context-mutated-by-process"))?;
    validate_snapshot(context, snapshot_id)?;
    let behavior_sha256 = framed(&[
        RESULT_DOMAIN,
        &observation.stdout,
        observation.stderr_sha256.as_bytes(),
        digest_of(&output_files)?.as_bytes(),
    ]);
    if report.outcome != "passed" || !report.behavior_observed {
        return Ok(IntentResult::Incomplete {
            disposition: RoutineNodeDisposition::Failed,
            failure_code: "MEDIATOR-REPORT-NOT-PASSED",
            started: true,
        });
    }
    let result = ResultArtifactWire {
        schema_version: "RoutineMediatedResultArtifact-v1".to_owned(),
        request_id: token.request_id().to_owned(),
        protocol_id: token.protocol_id().to_owned(),
        intent_id: token.intent().intent_id().to_owned(),
        node_id: token.intent().node_id().to_owned(),
        plan_order: token.intent().plan_order(),
        context_id: context.context_id().to_owned(),
        candidate_id: plan.binding().candidate_id().to_owned(),
        plan_id: plan.plan_id().to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        input_id: token.intent().input_id().to_owned(),
        tool_identity_sha256: token.intent().tool_identity_sha256().to_owned(),
        program_sha256: token.intent().program_sha256().to_owned(),
        environment_sha256: token.intent().environment_sha256().to_owned(),
        read_authority_sha256: token.intent().read_authority_sha256().to_owned(),
        dependency_results: dependencies.clone(),
        behavior_sha256,
        output_files: output_files.clone(),
    };
    let result_bytes = canonical(&result)?;
    let result_sha256 = sha256(&result_bytes);
    let mut reuse = ReuseArtifactWire {
        schema_version: "RoutineMediatedReuseArtifact-v1".to_owned(),
        state: "complete".to_owned(),
        protocol_id: token.protocol_id().to_owned(),
        intent_id: token.intent().intent_id().to_owned(),
        node_id: token.intent().node_id().to_owned(),
        plan_order: token.intent().plan_order(),
        context_id: context.context_id().to_owned(),
        candidate_id: plan.binding().candidate_id().to_owned(),
        plan_id: plan.plan_id().to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        input_id: token.intent().input_id().to_owned(),
        tool_identity_sha256: token.intent().tool_identity_sha256().to_owned(),
        program_sha256: token.intent().program_sha256().to_owned(),
        environment_sha256: token.intent().environment_sha256().to_owned(),
        read_authority_sha256: token.intent().read_authority_sha256().to_owned(),
        dependency_results: dependencies.clone(),
        output_files,
        result_artifact: result,
        result_artifact_sha256: result_sha256.clone(),
        mediator_witness_sha256: String::new(),
    };
    reuse.mediator_witness_sha256 = reuse_witness(&reuse)?;
    Ok(IntentResult::Executed(ExecutedArtifact {
        result_sha256,
        reuse_bytes: canonical(&reuse)?,
    }))
}

fn validate_grant(
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

fn reserve_grant(grant: &RoutineRootGrant) -> Result<AttemptReservation, RoutineError> {
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
    })
}

fn preflight_request(
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

fn validate_snapshot(context: &LiveContext, expected: &str) -> Result<(), RoutineError> {
    let current = LocalDirtyTree::capture(context)
        .map_err(|_| concurrent("mediator-dirty-snapshot-recapture-failed"))?;
    if current.snapshot_id() != expected {
        return Err(concurrent("mediator-dirty-snapshot-stale"));
    }
    Ok(())
}

fn validate_intent(
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

fn execution_environment(
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

fn parse_command_report(
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

fn index_reuse_inputs(
    input: RoutineReuseInput,
    request: &RoutineEffectRequest,
) -> Result<BTreeMap<String, Vec<u8>>, RoutineError> {
    let known = request
        .intents
        .iter()
        .map(|intent| intent.intent_id())
        .collect::<BTreeSet<_>>();
    let mut indexed = BTreeMap::new();
    for bytes in input.into_artifacts() {
        let Ok(wire) = serde_json::from_slice::<ReuseArtifactWire>(&bytes) else {
            continue;
        };
        if wire.protocol_id == request.protocol_id
            && known.contains(wire.intent_id.as_str())
            && wire.state != "complete"
        {
            return Err(mediator_error("mediator-ambiguous-started-artifact"));
        }
        if wire.state == "complete" && known.contains(wire.intent_id.as_str()) {
            if indexed.insert(wire.intent_id, bytes).is_some() {
                return Err(mediator_error("mediator-reuse-artifact-duplicated"));
            }
        }
    }
    Ok(indexed)
}

#[allow(clippy::too_many_arguments)]
fn verify_reuse_artifact(
    bytes: &[u8],
    context: &LiveContext,
    plan: &RoutinePlan,
    token: &RoutineMediatedIntent,
    snapshot_id: &str,
    dependencies: &BTreeMap<String, String>,
    outputs: &OutputConfinement,
) -> Result<Option<VerifiedReuseArtifact>, RoutineError> {
    let Ok(wire) = serde_json::from_slice::<ReuseArtifactWire>(bytes) else {
        return Ok(None);
    };
    if canonical(&wire)? != bytes || wire.state != "complete" {
        return Ok(None);
    }
    let artifact_sha256 = sha256(bytes);
    let authenticated = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .authenticated_artifacts
        .get(&artifact_sha256)
        .is_some_and(|witness| witness == &wire.mediator_witness_sha256);
    if !authenticated
        || wire.mediator_witness_sha256 != reuse_witness(&wire)?
        || wire.protocol_id != token.protocol_id()
        || wire.intent_id != token.intent().intent_id()
        || wire.node_id != token.intent().node_id()
        || wire.plan_order != token.intent().plan_order()
        || wire.context_id != context.context_id()
        || wire.candidate_id != plan.binding().candidate_id()
        || wire.plan_id != plan.plan_id()
        || wire.snapshot_id != snapshot_id
        || wire.input_id != token.intent().input_id()
        || wire.tool_identity_sha256 != token.intent().tool_identity_sha256()
        || wire.program_sha256 != token.intent().program_sha256()
        || wire.environment_sha256 != token.intent().environment_sha256()
        || wire.read_authority_sha256 != token.intent().read_authority_sha256()
        || wire.dependency_results != *dependencies
        || wire.result_artifact_sha256 != sha256(&canonical(&wire.result_artifact)?)
        || !result_matches_reuse(&wire)
        || outputs.capture()? != wire.output_files
    {
        return Ok(None);
    }
    Ok(Some(VerifiedReuseArtifact {
        wire,
        canonical_bytes: bytes.to_vec(),
    }))
}

fn result_matches_reuse(wire: &ReuseArtifactWire) -> bool {
    let result = &wire.result_artifact;
    result.schema_version == "RoutineMediatedResultArtifact-v1"
        && result.protocol_id == wire.protocol_id
        && result.intent_id == wire.intent_id
        && result.node_id == wire.node_id
        && result.plan_order == wire.plan_order
        && result.context_id == wire.context_id
        && result.candidate_id == wire.candidate_id
        && result.plan_id == wire.plan_id
        && result.snapshot_id == wire.snapshot_id
        && result.input_id == wire.input_id
        && result.tool_identity_sha256 == wire.tool_identity_sha256
        && result.program_sha256 == wire.program_sha256
        && result.environment_sha256 == wire.environment_sha256
        && result.read_authority_sha256 == wire.read_authority_sha256
        && result.dependency_results == wire.dependency_results
        && result.output_files == wire.output_files
        && valid(&result.behavior_sha256)
}

fn reuse_witness(wire: &ReuseArtifactWire) -> Result<String, RoutineError> {
    #[derive(Serialize)]
    struct Witness<'a> {
        domain: &'static str,
        schema_version: &'a str,
        state: &'a str,
        protocol_id: &'a str,
        intent_id: &'a str,
        node_id: &'a str,
        plan_order: usize,
        context_id: &'a str,
        candidate_id: &'a str,
        plan_id: &'a str,
        snapshot_id: &'a str,
        input_id: &'a str,
        tool_identity_sha256: &'a str,
        program_sha256: &'a str,
        environment_sha256: &'a str,
        read_authority_sha256: &'a str,
        dependency_results: &'a BTreeMap<String, String>,
        output_files: &'a BTreeMap<String, model::OutputFileRecord>,
        result_artifact_sha256: &'a str,
    }
    digest_of(&Witness {
        domain: "routine-mediated-reuse-witness-v1",
        schema_version: &wire.schema_version,
        state: &wire.state,
        protocol_id: &wire.protocol_id,
        intent_id: &wire.intent_id,
        node_id: &wire.node_id,
        plan_order: wire.plan_order,
        context_id: &wire.context_id,
        candidate_id: &wire.candidate_id,
        plan_id: &wire.plan_id,
        snapshot_id: &wire.snapshot_id,
        input_id: &wire.input_id,
        tool_identity_sha256: &wire.tool_identity_sha256,
        program_sha256: &wire.program_sha256,
        environment_sha256: &wire.environment_sha256,
        read_authority_sha256: &wire.read_authority_sha256,
        dependency_results: &wire.dependency_results,
        output_files: &wire.output_files,
        result_artifact_sha256: &wire.result_artifact_sha256,
    })
}

fn reconcile_internal(
    context: &LiveContext,
    plan: &RoutinePlan,
    authority: &RoutineMediationAuthority,
    nodes: &[RoutineNodeMediation],
) -> Result<(), RoutineError> {
    context
        .revalidate()
        .map_err(|_| concurrent("mediator-result-context-stale"))?;
    let current = RoutineBinding::from_live(context)?;
    if &current != plan.binding()
        || authority.binding != current
        || authority.plan_id != plan.plan_id()
        || authority.snapshot_id != plan.snapshot_id()
        || nodes.len() != authority.expected.len()
    {
        return Err(mediator_error("mediator-result-binding-invalid"));
    }
    for ((expected, node), check) in authority.expected.iter().zip(nodes).zip(plan.checks()) {
        if expected.intent_id != node.intent_id
            || expected.plan_order != node.plan_order
            || expected.node_id != node.node_id
            || check.node_id() != node.node_id
            || matches!(
                node.disposition,
                RoutineNodeDisposition::Executed | RoutineNodeDisposition::Reused
            ) != node.result_artifact_sha256.as_deref().is_some_and(valid)
            || matches!(
                node.disposition,
                RoutineNodeDisposition::Failed
                    | RoutineNodeDisposition::DependencyFailed
                    | RoutineNodeDisposition::Cancelled
            ) != node.failure_code.is_some()
        {
            return Err(mediator_error(
                "mediator-result-order-or-cardinality-invalid",
            ));
        }
    }
    authority.require_complete()?;
    Ok(())
}

fn success_node(
    token: &RoutineMediatedIntent,
    disposition: RoutineNodeDisposition,
    result_artifact_sha256: String,
) -> RoutineNodeMediation {
    RoutineNodeMediation {
        intent_id: token.intent().intent_id().to_owned(),
        node_id: token.intent().node_id().to_owned(),
        plan_order: token.intent().plan_order(),
        disposition,
        result_artifact_sha256: Some(result_artifact_sha256),
        failure_code: None,
    }
}

fn incomplete_node(
    token: &RoutineMediatedIntent,
    disposition: RoutineNodeDisposition,
    failure_code: impl Into<String>,
) -> RoutineNodeMediation {
    RoutineNodeMediation {
        intent_id: token.intent().intent_id().to_owned(),
        node_id: token.intent().node_id().to_owned(),
        plan_order: token.intent().plan_order(),
        disposition,
        result_artifact_sha256: None,
        failure_code: Some(failure_code.into()),
    }
}

fn authenticate_generated(values: Vec<(String, Vec<u8>)>) {
    let mut state = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (digest, bytes) in values {
        if let Ok(wire) = serde_json::from_slice::<ReuseArtifactWire>(&bytes) {
            state
                .authenticated_artifacts
                .insert(digest, wire.mediator_witness_sha256);
        }
    }
}

fn recovery_identity(grant_id: &str, protocol_id: &str, request_id: &str) -> String {
    framed(&[
        RECOVERY_DOMAIN,
        grant_id.as_bytes(),
        protocol_id.as_bytes(),
        request_id.as_bytes(),
    ])
}

fn mediator_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}

fn concurrent(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::ConcurrentMutation, cause, None)
}

#[cfg(test)]
type TestHook = Box<dyn FnOnce() + Send + 'static>;

#[cfg(test)]
fn test_hook() -> &'static Mutex<Option<TestHook>> {
    static HOOK: OnceLock<Mutex<Option<TestHook>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn post_spawn_test_hook() -> &'static Mutex<Option<TestHook>> {
    static HOOK: OnceLock<Mutex<Option<TestHook>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn finish_failure_test_hook() -> &'static Mutex<bool> {
    static HOOK: OnceLock<Mutex<bool>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(false))
}

#[cfg(test)]
pub(crate) fn set_test_mediator_pre_spawn_hook(hook: impl FnOnce() + Send + 'static) {
    *test_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
}

#[cfg(test)]
pub(crate) fn set_test_mediator_post_spawn_hook(hook: impl FnOnce() + Send + 'static) {
    *post_spawn_test_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
}

#[cfg(test)]
pub(crate) fn set_test_mediator_finish_failure() {
    *finish_failure_test_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
}

#[cfg(test)]
fn run_test_pre_spawn_hook() {
    if let Some(hook) = test_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
    {
        hook();
    }
}

#[cfg(test)]
fn run_test_post_spawn_hook() {
    if let Some(hook) = post_spawn_test_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
    {
        hook();
    }
}

#[cfg(test)]
fn run_test_finish_failure_hook(authority: &RoutineMediationAuthority) {
    let force = std::mem::take(
        &mut *finish_failure_test_hook()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    if force {
        authority
            .finish()
            .expect("test finish-failure hook requires complete authority");
    }
}

#[cfg(not(test))]
fn run_test_pre_spawn_hook() {}

#[cfg(not(test))]
fn run_test_post_spawn_hook() {}

#[cfg(not(test))]
fn run_test_finish_failure_hook(_authority: &RoutineMediationAuthority) {}

#[cfg(test)]
pub(crate) use filesystem::{set_test_output_capture_hook, set_test_read_source_capture_hook};
#[cfg(test)]
pub(crate) use process::{
    SetupFailurePoint as TestProcessSetupFailure, set_test_process_setup_failure, test_spawn_count,
};
