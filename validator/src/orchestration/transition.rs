use super::model::validate_digest;
use super::replay::{
    LeasePhase, Projection, is_active, require_owner, require_phase, require_root,
    require_unexpired,
};
use super::{Actor, EventKind, OrchestrationError, OrchestrationEvent, ReviewDecision, WorkGraph};

pub(crate) fn apply_lease_event(
    state: &mut Projection,
    item: &OrchestrationEvent,
    root: &Actor,
    graph: &WorkGraph,
    event: &EventKind,
    persisted_replay: bool,
) -> Result<(), OrchestrationError> {
    if let EventKind::CandidateRebound {
        observation,
        integration,
    } = event
    {
        return super::integration_transition::complete(
            state,
            item,
            root,
            graph,
            observation,
            integration,
            persisted_replay,
        );
    }
    if let EventKind::RootIntegrationStarted { intent } = event {
        return super::integration_transition::begin(state, item, root, intent);
    }
    if let EventKind::RootIntegrationObserved { observation } = event {
        return super::integration_transition::observe(
            state,
            item,
            root,
            observation,
            persisted_replay,
        );
    }
    if matches!(
        event,
        EventKind::EffectIntent { .. }
            | EventKind::EffectApplied { .. }
            | EventKind::EffectReconciled { .. }
    ) {
        return super::effect_transition::apply(state, item, root, event);
    }
    let lease_id = event.lease_id().ok_or(OrchestrationError::InvalidEvent)?;
    if let EventKind::ReviewRecorded {
        review,
        result_commitment_id,
        ..
    } = event
    {
        return record_review(state, item, lease_id, review, result_commitment_id);
    }
    let runtime = state
        .leases
        .get_mut(lease_id)
        .ok_or(OrchestrationError::InvalidLease)?;
    match event {
        EventKind::WorkStarted { .. } => {
            require_owner(&item.actor, runtime)?;
            require_unexpired(item, runtime)?;
            require_phase(&runtime.phase, |phase| matches!(phase, LeasePhase::Granted))?;
            runtime.phase = LeasePhase::Running;
        }
        EventKind::Heartbeat { .. } => {
            require_owner(&item.actor, runtime)?;
            if !matches!(runtime.phase, LeasePhase::Granted | LeasePhase::Running)
                || item.logical_tick > runtime.deadline_tick
            {
                return Err(OrchestrationError::InvalidTransition);
            }
            runtime.heartbeat_tick = item.logical_tick;
        }
        EventKind::WorkerSubmitted {
            commitment,
            result_commitment_id,
            ..
        } => {
            require_owner(&item.actor, runtime)?;
            require_unexpired(item, runtime)?;
            commitment.validate()?;
            validate_digest(result_commitment_id)?;
            if commitment.commitment_id()? != *result_commitment_id
                || commitment.binding != item.binding
                || commitment.node_id != runtime.spec.node_id
                || commitment.lease_id != runtime.spec.lease_id
            {
                return Err(OrchestrationError::InvalidWorkerResult);
            }
            require_phase(&runtime.phase, |phase| matches!(phase, LeasePhase::Running))?;
            runtime.phase = LeasePhase::Submitted {
                commitment: commitment.clone(),
                result_commitment_id: result_commitment_id.clone(),
            };
        }
        EventKind::ReviewAssigned { reviewer, .. } => {
            require_root(&item.actor, root)?;
            if reviewer == &runtime.spec.owner {
                return Err(OrchestrationError::ReviewerNotIndependent);
            }
            let LeasePhase::Submitted {
                commitment,
                result_commitment_id,
            } = &runtime.phase
            else {
                return Err(OrchestrationError::InvalidTransition);
            };
            runtime.phase = LeasePhase::AwaitingReview {
                commitment: commitment.clone(),
                result_commitment_id: result_commitment_id.clone(),
                reviewer: reviewer.clone(),
            };
        }
        EventKind::ReviewRecorded { .. } => return Err(OrchestrationError::InvalidEvent),
        EventKind::RetryScheduled {
            new_deadline_tick, ..
        } => {
            require_root(&item.actor, root)?;
            require_phase(&runtime.phase, |phase| {
                matches!(phase, LeasePhase::NeedsRetry)
            })?;
            if runtime.retries >= runtime.spec.max_retries
                || *new_deadline_tick <= item.logical_tick
            {
                return Err(OrchestrationError::RetryExhausted);
            }
            runtime.retries += 1;
            runtime.deadline_tick = *new_deadline_tick;
            runtime.phase = LeasePhase::Granted;
        }
        EventKind::LeaseCancelled { .. } => {
            require_root(&item.actor, root)?;
            if !is_active(&runtime.phase) {
                return Err(OrchestrationError::InvalidTransition);
            }
            runtime.phase = LeasePhase::Cancelled;
            state.active_registry.revoke(lease_id);
        }
        EventKind::RootAccepted { proposal } => {
            require_root(&item.actor, root)?;
            proposal.validate()?;
            let LeasePhase::ReviewedPass {
                commitment,
                result_commitment_id,
                review_id,
            } = &runtime.phase
            else {
                return Err(OrchestrationError::InvalidTransition);
            };
            if proposal.binding != item.binding
                || proposal.lease_id != lease_id
                || proposal.commitment()? != *commitment
                || &proposal.result_commitment_id != result_commitment_id
                || &proposal.review_id != review_id
            {
                return Err(OrchestrationError::InvalidReview);
            }
            runtime.phase = LeasePhase::Accepted {
                proposal_id: proposal.proposal_id()?,
                commitment: commitment.clone(),
                result_commitment_id: result_commitment_id.clone(),
            };
        }
        EventKind::CandidateRebound { .. }
        | EventKind::RootIntegrationStarted { .. }
        | EventKind::RootIntegrationObserved { .. }
        | EventKind::EffectIntent { .. }
        | EventKind::EffectApplied { .. }
        | EventKind::EffectReconciled { .. } => return Err(OrchestrationError::InvalidEvent),
        _ => return Err(OrchestrationError::InvalidEvent),
    }
    Ok(())
}

fn record_review(
    state: &mut Projection,
    item: &OrchestrationEvent,
    lease_id: &str,
    review: &super::ReviewRecord,
    event_commitment_id: &str,
) -> Result<(), OrchestrationError> {
    review.validate()?;
    let runtime = state
        .leases
        .get_mut(lease_id)
        .ok_or(OrchestrationError::InvalidLease)?;
    let LeasePhase::AwaitingReview {
        commitment,
        result_commitment_id,
        reviewer,
    } = &runtime.phase
    else {
        return Err(OrchestrationError::InvalidTransition);
    };
    if &item.actor != reviewer
        || review.reviewer != reviewer.as_str()
        || review.worker != runtime.spec.owner.as_str()
        || review.binding != item.binding
        || review.result_id != commitment.result_id
        || review.result_commitment_id != *result_commitment_id
        || event_commitment_id != result_commitment_id
        || commitment.commitment_id()? != *result_commitment_id
    {
        return Err(OrchestrationError::ReviewerNotIndependent);
    }
    let review_id = review.review_id()?;
    runtime.phase = match review.decision {
        ReviewDecision::Pass => LeasePhase::ReviewedPass {
            commitment: commitment.clone(),
            result_commitment_id: result_commitment_id.clone(),
            review_id,
        },
        ReviewDecision::Rework => LeasePhase::NeedsRetry,
        ReviewDecision::Reject => LeasePhase::Cancelled,
    };
    if review.decision == ReviewDecision::Reject {
        state.active_registry.revoke(lease_id);
    }
    Ok(())
}
