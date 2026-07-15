use super::*;

pub(crate) fn bind_read_sources(
    root: &Path,
    sources: &[RepoPath],
) -> Result<Vec<RoutineReadSource>, RoutineError> {
    if sources.is_empty() {
        return Ok(Vec::new());
    }
    let root = RootAnchor::open(root)?;
    ReadConfinement::bind_records(&root, sources)
}

pub(crate) fn validate_read_sources(
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
pub(crate) struct MediatorRegistry {
    pub(crate) consumed_grants: BTreeSet<String>,
    pub(crate) non_durable_authenticated_artifacts: BTreeMap<String, String>,
    pub(crate) ambiguous_protocols: BTreeMap<String, String>,
    pub(crate) active_protocols: BTreeMap<String, String>,
    pub(crate) failure_records: BTreeMap<String, ReservationFailureEvidence>,
}

pub(crate) fn registry() -> &'static Mutex<MediatorRegistry> {
    static REGISTRY: OnceLock<Mutex<MediatorRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(MediatorRegistry::default()))
}

pub(crate) fn release_active(state: &mut MediatorRegistry, protocol_id: &str, grant_id: &str) {
    if state.active_protocols.get(protocol_id).map(String::as_str) == Some(grant_id) {
        state.active_protocols.remove(protocol_id);
    }
}

#[derive(Serialize)]
pub(crate) struct GrantPayload<'a> {
    pub(crate) domain: &'static str,
    pub(crate) session_id: &'a str,
    pub(crate) request_id: &'a str,
    pub(crate) protocol_id: &'a str,
    pub(crate) context_id: &'a str,
    pub(crate) candidate_id: &'a str,
    pub(crate) plan_id: &'a str,
    pub(crate) snapshot_id: &'a str,
    pub(crate) allowed_output_scopes: &'a [RepoPath],
    pub(crate) recovery_for: Option<&'a str>,
}

pub(crate) fn grant_identity(grant: &RoutineRootGrant) -> Result<String, RoutineError> {
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

pub(crate) fn grant_seal(grant: &RoutineRootGrant) -> Result<String, RoutineError> {
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
    publisher: Option<&dyn RoutineArtifactPublisher>,
) -> Result<RoutineMediationResult, RoutineError> {
    match prepared {
        PreparedRoutineExecution::NoOp(projection) => {
            mediate_noop(context, plan, projection, grant, reuse)
        }
        PreparedRoutineExecution::Effect(request) => mediate_effect(
            context,
            plan,
            request,
            grant,
            cancellation,
            reuse,
            publisher,
        ),
    }
}
