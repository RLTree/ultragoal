use super::replay::{LeasePhase, Projection, is_active, require_root};
use super::{
    Actor, IntegrationDisposition, LeaseRegistry, OrchestrationError, OrchestrationEvent,
    RootIntegrationIntent, RootIntegrationObservation, RootIntegrationReceipt, WorkGraph,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn begin(
    state: &mut Projection,
    item: &OrchestrationEvent,
    root: &Actor,
    intent: &RootIntegrationIntent,
) -> Result<(), OrchestrationError> {
    require_root(&item.actor, root)?;
    intent.validate()?;
    if intent.base_binding != item.binding
        || intent.root_actor != item.actor.as_str()
        || state.integration_intent.is_some()
        || !state.pending_effects.is_empty()
    {
        return Err(OrchestrationError::IntegrationAmbiguous);
    }
    let mut expected_changes = BTreeMap::new();
    for (lease_id, expected_proposal) in &intent.accepted_proposals {
        let runtime = state
            .leases
            .get(lease_id)
            .ok_or(OrchestrationError::InvalidLease)?;
        let LeasePhase::Accepted {
            proposal_id,
            commitment,
            ..
        } = &runtime.phase
        else {
            return Err(OrchestrationError::InvalidTransition);
        };
        if proposal_id != expected_proposal {
            return Err(OrchestrationError::InvalidReview);
        }
        for (path, digest) in &commitment.expected_root_changes {
            if expected_changes
                .insert(path.clone(), digest.clone())
                .is_some()
            {
                return Err(OrchestrationError::DuplicateOutput);
            }
        }
    }
    if expected_changes != intent.expected_digests {
        return Err(OrchestrationError::InvalidReview);
    }
    state.integration_intent = Some(intent.clone());
    state.integration_observation = None;
    Ok(())
}

pub(crate) fn observe(
    state: &mut Projection,
    item: &OrchestrationEvent,
    root: &Actor,
    observation: &RootIntegrationObservation,
    persisted_replay: bool,
) -> Result<(), OrchestrationError> {
    require_root(&item.actor, root)?;
    let intent = state
        .integration_intent
        .as_ref()
        .ok_or(OrchestrationError::IntegrationAmbiguous)?;
    observation.validate_for(intent, persisted_replay)?;
    state.integration_observation = Some(observation.clone());
    Ok(())
}

pub(crate) fn complete(
    state: &mut Projection,
    item: &OrchestrationEvent,
    root: &Actor,
    graph: &WorkGraph,
    observation: &RootIntegrationObservation,
    integration: &RootIntegrationReceipt,
    persisted_replay: bool,
) -> Result<(), OrchestrationError> {
    require_root(&item.actor, root)?;
    integration.validate()?;
    let intent = state
        .integration_intent
        .as_ref()
        .ok_or(OrchestrationError::IntegrationAmbiguous)?;
    observation.validate_for(intent, persisted_replay)?;
    if observation.disposition != IntegrationDisposition::Full
        || intent.base_binding != item.binding
        || intent.expected_digests != integration.applied_changes
    {
        return Err(OrchestrationError::IntegrationAmbiguous);
    }
    let accepted_ids: BTreeSet<_> = integration.accepted_leases.keys().cloned().collect();
    if accepted_ids != intent.accepted_proposals.keys().cloned().collect() {
        return Err(OrchestrationError::InvalidReview);
    }
    let expected_invalidations: BTreeSet<_> = state
        .leases
        .iter()
        .filter(|(id, other)| !accepted_ids.contains(*id) && is_active(&other.phase))
        .map(|(id, _)| id.clone())
        .collect();
    let mut completed_evidence = state.completed_evidence.clone();
    let mut accepted_nodes = BTreeSet::new();
    let mut expected_request_digests = BTreeSet::new();
    let mut expected_applied_changes = BTreeMap::new();
    for (lease_id, accepted) in &integration.accepted_leases {
        let runtime = state
            .leases
            .get(lease_id)
            .ok_or(OrchestrationError::InvalidLease)?;
        let LeasePhase::Accepted {
            proposal_id,
            commitment,
            result_commitment_id,
        } = &runtime.phase
        else {
            return Err(OrchestrationError::InvalidTransition);
        };
        if accepted.node_id != runtime.spec.node_id
            || accepted.proposal_id != *proposal_id
            || commitment.commitment_id()? != *result_commitment_id
            || accepted.requested_root_changes_digest != commitment.requested_root_changes_digest
            || accepted.requested_root_change_count != commitment.requested_root_change_count
            || !accepted_nodes.insert(accepted.node_id.clone())
        {
            return Err(OrchestrationError::InvalidReview);
        }
        for (path, digest) in &commitment.expected_root_changes {
            if expected_applied_changes
                .insert(path.clone(), digest.clone())
                .is_some()
            {
                return Err(OrchestrationError::DuplicateOutput);
            }
        }
        expected_request_digests.insert(commitment.requested_root_changes_digest.clone());
        completed_evidence.insert(commitment.node_id.clone(), accepted.proposal_id.clone());
    }
    super::validate_root_change_map(&expected_applied_changes)?;
    let completed_nodes: BTreeSet<_> = completed_evidence.keys().cloned().collect();
    if integration.base_binding != item.binding
        || integration.integrated_binding == item.binding
        || integration.root_actor != item.actor.as_str()
        || integration.reconciled_request_digests != expected_request_digests
        || integration.applied_changes != expected_applied_changes
        || integration.invalidated_lease_ids != expected_invalidations
        || integration.integrated_bootstrap.observed_tick != item.logical_tick
        || integration.integrated_bootstrap.completed_nodes != completed_evidence
        || !graph.dependency_closed(&completed_nodes)?
    {
        return Err(OrchestrationError::InvalidReview);
    }
    for (id, other) in &mut state.leases {
        other.phase = if accepted_ids.contains(id) {
            LeasePhase::Completed
        } else if expected_invalidations.contains(id) {
            LeasePhase::Cancelled
        } else {
            other.phase.clone()
        };
    }
    state.active_registry = LeaseRegistry::default();
    state.completed_nodes = completed_nodes;
    state.completed_evidence = integration.integrated_bootstrap.completed_nodes.clone();
    state.available_tools = integration.integrated_bootstrap.available_tools.clone();
    state.satisfied_prerequisites = integration
        .integrated_bootstrap
        .satisfied_prerequisites
        .clone();
    state.current_binding = Some(integration.integrated_binding.clone());
    state.integration_intent = None;
    state.integration_observation = None;
    Ok(())
}
