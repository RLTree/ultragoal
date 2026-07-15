use super::*;

pub(crate) const GRANT_SEAL_DOMAIN: &[u8] = b"routine-root-grant-seal-v1";
pub(crate) const RESULT_DOMAIN: &[u8] = b"routine-mediated-result-v1";
pub(crate) const RECOVERY_DOMAIN: &[u8] = b"routine-mediated-recovery-v1";
pub(crate) const MEDIATOR_SUPPORT_LIMIT: &str = "internal macOS single-process routine mediation evidence only; grant replay, reuse authentication, ambiguity recovery, and artifacts are process-local; executable paths must be immutable to this user; after sandbox activation only the exact pinned executable identity may execute, unbound file reads are denied, explicitly bound worktree-relative regular-file reads are identity/content/ctime revalidated, immutable system runtime roots remain policy-authorized, post-activation file-backed executable mapping is limited to immutable system-library roots, and process-fork kills the runner; external interpreted sources, startup-loader environments, executable trampolines, different-object aliases, shebang scripts, descriptor aliases, user-owned executable mappings, and multi-process runners are unsupported; canonical root issuance, durable persistence, public dispatch, installed behavior, and claim decisions remain absent";
pub(crate) const PRODUCTION_SUPPORT_LIMIT: &str = "source-local canonical routine planning, clean no-op, rust-source-syntax execution, exact durable reuse, conservative fallback, cancellation, bounded interruption recovery, and parent-authenticated observation; arbitrary programs, child-authored results, unbound fallback, installed behavior, representative product journeys, readiness, release, and completion remain unavailable";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DurableSettlement {
    Complete,
    Failed,
    Cancelled,
    Incomplete,
}

/// Sealed bridge between the process mediator and the durable production
/// authority. Implementations live only in the sibling production issuer.
pub(crate) trait DurableAttemptAuthority: Send + Sync {
    fn validate_reserved(&self) -> Result<(), RoutineError>;
    fn stage_program(&self, program: &PinnedExecutable) -> Result<StagedProgram, RoutineError>;
    fn cleanup_staged(&self, staged: &StagedProgram) -> Result<(), RoutineError>;
    fn prepare_spawn(&self) -> Result<(), RoutineError>;
    fn stage_success(&self, artifacts: &BTreeMap<String, String>) -> Result<(), RoutineError>;
    fn settle(
        &self,
        outcome: DurableSettlement,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError>;
    fn authenticates_artifact(&self, digest: &str, witness: &str) -> Result<bool, RoutineError>;
    fn recovery_is_durable(&self) -> bool;
    fn reuse_only(&self) -> bool;
}

pub(crate) trait RoutineArtifactPublisher {
    fn publish(&self, artifacts: &[Vec<u8>]) -> Result<(), RoutineError>;
}

#[derive(Clone)]
pub(crate) struct ProductionGrantBinding {
    pub(crate) session_id: String,
    pub(crate) request_id: String,
    pub(crate) protocol_id: String,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plan_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) allowed_output_scopes: Vec<RepoPath>,
    pub(crate) recovery_for: Option<String>,
}

pub(crate) fn production_grant_identity(
    spec: &ProductionGrantBinding,
) -> Result<String, RoutineError> {
    let grant = RoutineRootGrant {
        grant_id: String::new(),
        session_id: spec.session_id.clone(),
        request_id: spec.request_id.clone(),
        protocol_id: spec.protocol_id.clone(),
        context_id: spec.context_id.clone(),
        candidate_id: spec.candidate_id.clone(),
        plan_id: spec.plan_id.clone(),
        snapshot_id: spec.snapshot_id.clone(),
        allowed_output_scopes: spec.allowed_output_scopes.clone(),
        recovery_for: spec.recovery_for.clone(),
        seal: String::new(),
        durable: None,
    };
    grant_identity(&grant)
}

pub(crate) fn production_recovery_identity(
    grant_id: &str,
    protocol_id: &str,
    request_id: &str,
) -> String {
    recovery_identity(grant_id, protocol_id, request_id)
}

pub(crate) fn issue_production_grant(
    spec: ProductionGrantBinding,
    durable: Arc<dyn DurableAttemptAuthority>,
) -> Result<RoutineRootGrant, RoutineError> {
    let mut grant = RoutineRootGrant {
        grant_id: String::new(),
        session_id: spec.session_id,
        request_id: spec.request_id,
        protocol_id: spec.protocol_id,
        context_id: spec.context_id,
        candidate_id: spec.candidate_id,
        plan_id: spec.plan_id,
        snapshot_id: spec.snapshot_id,
        allowed_output_scopes: spec.allowed_output_scopes,
        recovery_for: spec.recovery_for,
        seal: String::new(),
        durable: Some(durable),
    };
    grant.grant_id = grant_identity(&grant)?;
    grant.seal = grant_seal(&grant)?;
    Ok(grant)
}

pub(crate) fn preflight_production_request(
    context: &LiveContext,
    plan: &RoutinePlan,
    request: &RoutineEffectRequest,
) -> Result<(), RoutineError> {
    preflight_request(context, plan, request)
}

/// Exact, parsed commitments carried from zero-write input validation into the
/// durable ledger's read-only reuse authentication. These values are not
/// authority: only the ledger may turn them into an opaque, one-use
/// preauthorization bound to its current Complete record.
pub(crate) struct ProductionReuseClaim {
    pub(crate) protocol_id: String,
    pub(crate) intent_id: String,
    pub(crate) artifact_sha256: String,
    pub(crate) result_artifact_sha256: String,
    pub(crate) mediator_witness_sha256: String,
}

pub(crate) struct PreflightedProductionReuse {
    pub(crate) input: RoutineReuseInput,
    pub(crate) claims: Vec<ProductionReuseClaim>,
}

impl PreflightedProductionReuse {
    pub(crate) fn is_empty(&self) -> bool {
        self.claims.is_empty()
    }

    pub(crate) fn into_parts(self) -> (RoutineReuseInput, Vec<ProductionReuseClaim>) {
        (self.input, self.claims)
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
        return Ok(PreflightedProductionReuse {
            input,
            claims: Vec::new(),
        });
    }
    let known = request
        .intents
        .iter()
        .map(|intent| (intent.intent_id(), intent))
        .collect::<BTreeMap<_, _>>();
    let mut supplied = BTreeSet::new();
    let mut claims = Vec::with_capacity(input.artifacts().len());
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
        claims.push(ProductionReuseClaim {
            protocol_id: wire.protocol_id,
            intent_id: wire.intent_id,
            artifact_sha256: sha256(bytes),
            result_artifact_sha256: wire.result_artifact_sha256,
            mediator_witness_sha256: wire.mediator_witness_sha256,
        });
    }
    if require_complete_set && supplied.len() != known.len() {
        return Err(mediator_error("mediator-production-reuse-input-incomplete"));
    }
    claims.sort_by(|left, right| left.intent_id.cmp(&right.intent_id));
    Ok(PreflightedProductionReuse { input, claims })
}
