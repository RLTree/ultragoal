use super::*;

pub(crate) const RESULT_DOMAIN: &[u8] = b"routine-mediated-result-v1";
pub(crate) const RECOVERY_DOMAIN: &[u8] = b"routine-mediated-recovery-v1";
pub(crate) const MEDIATOR_SUPPORT_LIMIT: &str = "internal macOS single-process routine mediation evidence only; grant replay, reuse authentication, ambiguity recovery, and artifacts are process-local; the parent kernel-suspends each launch and matches its loaded vnode to the exact pinned executable descriptor before resume, so a user-writable ancestor name cannot select executed bytes; the exact child activates the fixed sandbox before frame evaluation, unbound file reads are denied, explicitly bound worktree-relative regular-file reads are identity/content/ctime revalidated, immutable system runtime roots remain policy-authorized, post-activation file-backed executable mapping is limited to immutable system-library roots, and process-fork kills the runner; external interpreted sources, startup-loader environments, executable trampolines, different-object aliases, shebang scripts, descriptor aliases, user-owned executable mappings, and multi-process runners are unsupported; canonical root issuance, durable persistence, public dispatch, installed behavior, and claim decisions remain absent";
pub(crate) const PRODUCTION_SUPPORT_LIMIT: &str = "source-local canonical routine planning, clean no-op, one fresh single-process rust-source-syntax execution with parent-authenticated observation and durable terminal publication, exact same-binding fresh-process reuse from an authenticated Complete ledger state, plus one public reservation interruption and exact fresh-process no-effect reconciliation through a private host checkpoint bound to the current target, context, candidate, plan, snapshot, attempt, grant, and authenticated ledger head; fallback, started-or-unknown recovery, rollback resistance beyond the reserved no-effect case, installed behavior, representative product journeys, readiness, release, and completion remain unavailable";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DurableSettlement {
    Complete,
    Failed,
    Cancelled,
    Incomplete,
}

pub(crate) fn preflight_production_request(
    context: &LiveContext,
    plan: &RoutinePlan,
    request: &RoutineEffectRequest,
) -> Result<(), RoutineError> {
    preflight_request_without_outputs(context, plan, request)
}

/// Exact, parsed commitments carried from zero-write input validation into the
/// durable ledger's read-only reuse authentication. These values are not
/// authority: only the ledger may turn them into an opaque, one-use
/// preauthorization bound to its current Complete record.
pub(crate) struct PreflightedProductionReuse {
    pub(crate) input: RoutineReuseInput,
}

impl PreflightedProductionReuse {
    pub(crate) fn is_empty(&self) -> bool {
        self.input.is_empty()
    }
}

/// Validates caller-supplied production reuse bytes before durable authority is
/// opened or reserved. Production input is an exact request-scoped set, not a
/// permissive cache bag: malformed, non-canonical, foreign, duplicate, and
/// incomplete exact-reuse sets fail closed without reaching the ledger.
pub(crate) fn preflight_production_reuse_input(
    input: RoutineReuseInput,
    request: &RoutineEffectRequest,
    require_complete_set: bool,
) -> Result<PreflightedProductionReuse, RoutineError> {
    if input.is_empty() {
        return Ok(PreflightedProductionReuse { input });
    }
    let known = request
        .intents
        .iter()
        .map(|intent| (intent.intent_id(), intent))
        .collect::<BTreeMap<_, _>>();
    let mut supplied = BTreeSet::new();
    for bytes in input.artifacts() {
        let wire: ReuseArtifactWire = serde_json::from_slice(bytes)
            .map_err(|_| mediator_error("mediator-production-reuse-input-malformed"))?;
        let intent = known.get(wire.intent_id.as_str());
        if canonical(&wire)?.as_slice() != bytes.as_slice()
            || wire.schema_version != "RoutineMediatedReuseArtifact-v2"
            || wire.state != "complete"
            || wire.protocol_id != request.protocol_id
            || intent.is_none()
        {
            return Err(mediator_error(
                "mediator-production-reuse-input-binding-invalid",
            ));
        }
        let intent = intent.expect("checked exact request intent");
        if wire.node_id != intent.node_id()
            || wire.behavior_id != intent.behavior_id()
            || wire.plan_order != intent.plan_order()
            || wire.context_id != request.context_id()
            || wire.candidate_id != request.candidate_id()
            || wire.plan_id != request.plan_id()
            || wire.snapshot_id != request.snapshot_id
            || wire.input_id != intent.input_id()
            || wire.tool_identity_sha256 != intent.tool_identity_sha256()
            || wire.program_sha256 != intent.program_sha256()
            || wire.environment_sha256 != intent.environment_sha256()
            || wire.read_authority_sha256 != intent.read_authority_sha256()
        {
            return Err(mediator_error(
                "mediator-production-reuse-input-binding-invalid",
            ));
        }
        if wire.mediator_witness_sha256 != reuse_witness(&wire)?
            || wire.result_artifact_sha256 != sha256(&canonical(&wire.result_artifact)?)
            || !result_matches_reuse(&wire)
        {
            return Err(mediator_error(
                "mediator-production-reuse-not-authenticated",
            ));
        }
        if !supplied.insert(wire.intent_id.clone()) {
            return Err(mediator_error("mediator-production-reuse-input-duplicated"));
        }
    }
    if require_complete_set && supplied.len() != known.len() {
        return Err(mediator_error("mediator-production-reuse-input-incomplete"));
    }
    Ok(PreflightedProductionReuse { input })
}
