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
