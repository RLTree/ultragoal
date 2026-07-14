#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LeasePhase {
    Granted,
    Running,
    Submitted {
        commitment: ResultCommitment,
        result_commitment_id: String,
    },
    AwaitingReview {
        commitment: ResultCommitment,
        result_commitment_id: String,
        reviewer: Actor,
    },
    ReviewedPass {
        commitment: ResultCommitment,
        result_commitment_id: String,
        review_id: String,
    },
    NeedsRetry,
    Accepted {
        proposal_id: String,
        commitment: ResultCommitment,
        result_commitment_id: String,
    },
    Cancelled,
    Completed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LeaseRuntime {
    pub spec: LeaseSpec,
    pub phase: LeasePhase,
    pub heartbeat_tick: u64,
    pub deadline_tick: u64,
    pub retries: u8,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Projection {
    pub current_binding: Option<Binding>,
    pub leases: BTreeMap<String, LeaseRuntime>,
    pub active_registry: LeaseRegistry,
    pub completed_nodes: BTreeSet<String>,
    pub completed_evidence: BTreeMap<String, String>,
    pub available_tools: BTreeMap<String, String>,
    pub satisfied_prerequisites: BTreeMap<String, String>,
    pub bootstrapped: bool,
    pub root_interrupted: bool,
    pub pending_effects: BTreeMap<String, EffectRequest>,
    pub settled_effects: BTreeSet<String>,
    pub integration_intent: Option<RootIntegrationIntent>,
    pub integration_observation: Option<RootIntegrationObservation>,
}

impl Projection {
    pub fn active_nodes(&self) -> BTreeSet<String> {
        self.leases
            .values()
            .filter(|runtime| is_active(&runtime.phase))
            .map(|runtime| runtime.spec.node_id.clone())
            .collect()
    }
}

impl EventLog {
    pub(crate) fn apply_next(
        &self,
        current: &Projection,
        item: &OrchestrationEvent,
        binding: &Binding,
        root: &Actor,
        graph: &WorkGraph,
        policy: &ScopePolicy,
    ) -> Result<Projection, OrchestrationError> {
        item.verify_identity()?;
        if item.sequence != self.0.len() as u64
            || item.prior_event_id.as_deref() != self.0.last().map(|event| event.event_id.as_str())
            || &item.binding != binding
            || current.current_binding.as_ref() != Some(binding)
            || self
                .0
                .last()
                .is_some_and(|prior| item.logical_tick < prior.logical_tick)
        {
            return Err(OrchestrationError::ReplayMismatch);
        }
        let mut next = current.clone();
        apply_event(&mut next, item, root, graph, policy, false)?;
        Ok(next)
    }

    #[rustfmt::skip]
    pub(crate) fn replay(&self, final_binding: &Binding, root: &Actor, graph: &WorkGraph, policy: &ScopePolicy) -> Result<Projection, OrchestrationError> { self.replay_with_trust(final_binding, root, graph, policy, false) }

    #[rustfmt::skip]
    pub(crate) fn replay_persisted(&self, final_binding: &Binding, root: &Actor, graph: &WorkGraph, policy: &ScopePolicy) -> Result<Projection, OrchestrationError> { self.replay_with_trust(final_binding, root, graph, policy, true) }

    fn replay_with_trust(
        &self,
        final_binding: &Binding,
        root: &Actor,
        graph: &WorkGraph,
        policy: &ScopePolicy,
        persisted_replay: bool,
    ) -> Result<Projection, OrchestrationError> {
        if self.0.len() > 16_384 {
            return Err(OrchestrationError::ResourceLimit);
        }
        let mut projection = Projection::default();
        let mut prior: Option<&str> = None;
        let mut prior_tick = 0;
        for (index, event) in self.0.iter().enumerate() {
            event.verify_identity()?;
            if event.sequence != index as u64
                || event.prior_event_id.as_deref() != prior
                || projection
                    .current_binding
                    .as_ref()
                    .is_some_and(|current| &event.binding != current)
                || (index > 0 && event.logical_tick < prior_tick)
            {
                return Err(OrchestrationError::ReplayMismatch);
            }
            apply_event(
                &mut projection,
                event,
                root,
                graph,
                policy,
                persisted_replay,
            )?;
            prior = Some(&event.event_id);
            prior_tick = event.logical_tick;
        }
        if !projection.bootstrapped {
            return Err(OrchestrationError::InvalidTransition);
        }
        if projection.current_binding.as_ref() != Some(final_binding) {
            return Err(OrchestrationError::ReplayMismatch);
        }
        Ok(projection)
    }
}
