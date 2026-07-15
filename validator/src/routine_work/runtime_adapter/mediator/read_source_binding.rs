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
}

pub(crate) fn registry() -> &'static Mutex<MediatorRegistry> {
    static REGISTRY: OnceLock<Mutex<MediatorRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(MediatorRegistry::default()))
}

/// Process-local attempt reservation guarding the gap between grant
/// consumption and final reconciliation.
///
/// A protocol is reserved before mediation begins. The explicit reservation
/// lifecycle records every later transition; leaving scope never changes
/// authority state.
pub(crate) struct AttemptReservation {
    pub(crate) protocol_id: String,
    pub(crate) grant_id: String,
    pub(crate) recovery_marker: String,
    pub(crate) prior_recovery_marker: Option<String>,
    pub(crate) started: Cell<bool>,
    pub(crate) settled: Cell<bool>,
    pub(crate) durable: Option<Arc<dyn DurableAttemptAuthority>>,
    pub(crate) staged: RefCell<Vec<StagedProgram>>,
}

impl AttemptReservation {
    fn expected_ambiguity(&self) -> Option<&String> {
        if self.started.get() {
            Some(&self.recovery_marker)
        } else {
            self.prior_recovery_marker.as_ref()
        }
    }

    fn clear_exact_ambiguity(&self, state: &mut MediatorRegistry) {
        if self.expected_ambiguity().is_some_and(|expected| {
            state.ambiguous_protocols.get(&self.protocol_id) == Some(expected)
        }) {
            state.ambiguous_protocols.remove(&self.protocol_id);
        }
    }

    pub(crate) fn reuse_only(&self) -> bool {
        self.durable
            .as_ref()
            .is_some_and(|durable| durable.reuse_only())
    }

    pub(crate) fn prepare_spawn(&self) -> Result<(), RoutineError> {
        if let Some(durable) = &self.durable {
            durable.prepare_spawn()?;
        }
        Ok(())
    }

    pub(crate) fn mark_started(&self) -> Result<(), RoutineError> {
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state
            .ambiguous_protocols
            .get(&self.protocol_id)
            .is_some_and(|marker| marker != &self.recovery_marker)
        {
            return Err(mediator_error("mediator-recovery-marker-conflict"));
        }
        state
            .ambiguous_protocols
            .insert(self.protocol_id.clone(), self.recovery_marker.clone());
        self.started.set(true);
        Ok(())
    }

    pub(crate) fn stage_success(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        if let Some(durable) = &self.durable {
            durable.stage_success(artifacts)?;
        }
        Ok(())
    }

    pub(crate) fn settle_success(
        &self,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        if let Some(durable) = &self.durable {
            durable.settle(DurableSettlement::Complete, artifacts)?;
        }
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        release_active(&mut state, &self.protocol_id, &self.grant_id);
        self.clear_exact_ambiguity(&mut state);
        self.settled.set(true);
        Ok(())
    }

    pub(crate) fn settle_incomplete(
        &self,
        outcome: DurableSettlement,
    ) -> Result<Option<String>, RoutineError> {
        let durably_terminal = if let Some(durable) = &self.durable {
            durable.settle(outcome, &BTreeMap::new())?;
            true
        } else {
            false
        };
        let mut state = registry()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        release_active(&mut state, &self.protocol_id, &self.grant_id);
        if durably_terminal {
            self.clear_exact_ambiguity(&mut state);
        }
        let pending_marker = (!durably_terminal)
            .then(|| {
                self.expected_ambiguity()
                    .filter(|expected| {
                        state.ambiguous_protocols.get(&self.protocol_id) == Some(*expected)
                    })
                    .cloned()
            })
            .flatten();
        self.settled.set(true);
        Ok(pending_marker)
    }
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
