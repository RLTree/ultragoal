use super::event::ResultCommitment;
use super::{
    Actor, Binding, EffectRequest, EventKind, EventLog, LeaseRegistry, LeaseSpec,
    OrchestrationError, OrchestrationEvent, Principal, RootIntegrationIntent,
    RootIntegrationObservation, ScopePolicy, WorkGraph,
};
use std::collections::{BTreeMap, BTreeSet};

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

fn apply_event(
    state: &mut Projection,
    item: &OrchestrationEvent,
    root: &Actor,
    graph: &WorkGraph,
    policy: &ScopePolicy,
    persisted_replay: bool,
) -> Result<(), OrchestrationError> {
    if !state.bootstrapped && !matches!(item.event, EventKind::Bootstrapped { .. }) {
        return Err(OrchestrationError::InvalidTransition);
    }
    if state.root_interrupted && !matches!(item.event, EventKind::RootRecovered) {
        return Err(OrchestrationError::InvalidTransition);
    }
    match &item.event {
        EventKind::Bootstrapped { evidence } => {
            require_root(&item.actor, root)?;
            evidence.validate()?;
            let completed: BTreeSet<_> = evidence.completed_nodes.keys().cloned().collect();
            if state.bootstrapped
                || evidence.observed_tick != item.logical_tick
                || !graph.dependency_closed(&completed)?
            {
                return Err(OrchestrationError::InvalidTransition);
            }
            state.completed_nodes = completed;
            state.current_binding = Some(item.binding.clone());
            state.completed_evidence = evidence.completed_nodes.clone();
            state.available_tools = evidence.available_tools.clone();
            state.satisfied_prerequisites = evidence.satisfied_prerequisites.clone();
            state.bootstrapped = true;
        }
        EventKind::LeaseGranted { lease } => {
            require_root(&item.actor, root)?;
            let package = graph
                .package(&lease.node_id)
                .ok_or(OrchestrationError::UnknownNode)?;
            if state.completed_nodes.contains(&lease.node_id) {
                return Err(OrchestrationError::InvalidTransition);
            }
            if lease.issued_tick != item.logical_tick
                || lease.safety_class != package.safety_class
                || !lease.owned_scope.is_subset_of(&package.owned_scope)
                || !lease.read_paths.is_subset(&package.read_paths)
                || lease.prerequisite_evidence.dependency_nodes
                    != select_evidence(&state.completed_evidence, &package.dependencies)?
                || lease.prerequisite_evidence.required_tools
                    != select_evidence(&state.available_tools, &package.required_tools)?
                || lease.prerequisite_evidence.prerequisites
                    != select_evidence(&state.satisfied_prerequisites, &package.prerequisites)?
                || (lease.principal == Principal::Root && &lease.owner != root)
                || (lease.principal == Principal::Worker && &lease.owner == root)
            {
                return Err(OrchestrationError::InvalidLease);
            }
            state
                .active_registry
                .grant(lease.clone(), policy, &item.binding)?;
            if state.leases.contains_key(&lease.lease_id) {
                return Err(OrchestrationError::InvalidLease);
            }
            state.leases.insert(
                lease.lease_id.clone(),
                LeaseRuntime {
                    spec: lease.clone(),
                    phase: LeasePhase::Granted,
                    heartbeat_tick: item.logical_tick,
                    deadline_tick: lease.heartbeat_deadline_tick,
                    retries: 0,
                },
            );
        }
        EventKind::RootInterrupted => {
            require_root(&item.actor, root)?;
            state.root_interrupted = true;
        }
        EventKind::RootRecovered => {
            require_root(&item.actor, root)?;
            if !state.root_interrupted {
                return Err(OrchestrationError::InvalidTransition);
            }
            state.root_interrupted = false;
        }
        other => {
            super::transition::apply_lease_event(state, item, root, graph, other, persisted_replay)?
        }
    }
    Ok(())
}

fn select_evidence(
    available: &BTreeMap<String, String>,
    required: &BTreeSet<String>,
) -> Result<BTreeMap<String, String>, OrchestrationError> {
    required
        .iter()
        .map(|key| {
            available
                .get(key)
                .cloned()
                .map(|value| (key.clone(), value))
                .ok_or(OrchestrationError::InvalidLease)
        })
        .collect()
}

pub(crate) fn require_root(actor: &Actor, root: &Actor) -> Result<(), OrchestrationError> {
    if actor != root {
        return Err(OrchestrationError::RootOnlyScope);
    }
    Ok(())
}

pub(crate) fn is_active(phase: &LeasePhase) -> bool {
    !matches!(phase, LeasePhase::Cancelled | LeasePhase::Completed)
}

pub(crate) fn require_unexpired(
    item: &OrchestrationEvent,
    runtime: &LeaseRuntime,
) -> Result<(), OrchestrationError> {
    if item.logical_tick > runtime.deadline_tick {
        return Err(OrchestrationError::InvalidTransition);
    }
    Ok(())
}

pub(crate) fn require_owner(
    actor: &Actor,
    runtime: &LeaseRuntime,
) -> Result<(), OrchestrationError> {
    if actor != &runtime.spec.owner {
        return Err(OrchestrationError::InvalidTransition);
    }
    Ok(())
}

pub(crate) fn require_phase(
    phase: &LeasePhase,
    predicate: impl FnOnce(&LeasePhase) -> bool,
) -> Result<(), OrchestrationError> {
    if !predicate(phase) {
        return Err(OrchestrationError::InvalidTransition);
    }
    Ok(())
}
