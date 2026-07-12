use super::replay::{LeasePhase, Projection, require_owner, require_root, require_unexpired};
use super::{Actor, EventKind, OrchestrationError, OrchestrationEvent};

pub(crate) fn apply(
    state: &mut Projection,
    item: &OrchestrationEvent,
    root: &Actor,
    event: &EventKind,
) -> Result<(), OrchestrationError> {
    match event {
        EventKind::EffectIntent { lease_id, request } => {
            let runtime = state
                .leases
                .get(lease_id)
                .ok_or(OrchestrationError::InvalidLease)?;
            require_owner(&item.actor, runtime)?;
            require_unexpired(item, runtime)?;
            request.validate()?;
            if request.lease_id != *lease_id
                || request.binding != item.binding
                || !matches!(runtime.phase, LeasePhase::Running)
                || !runtime.spec.owned_scope.effects.contains(&request.effect)
                || state.pending_effects.contains_key(&request.operation_id)
                || state.settled_effects.contains(&request.operation_id)
            {
                return Err(OrchestrationError::EffectDenied);
            }
            state
                .pending_effects
                .insert(request.operation_id.clone(), request.clone());
        }
        EventKind::EffectApplied { lease_id, receipt } => {
            let runtime = state
                .leases
                .get(lease_id)
                .ok_or(OrchestrationError::InvalidLease)?;
            require_owner(&item.actor, runtime)?;
            let request = state
                .pending_effects
                .get(&receipt.operation_id)
                .ok_or(OrchestrationError::EffectDenied)?;
            if request.lease_id != *lease_id {
                return Err(OrchestrationError::EffectDenied);
            }
            receipt.validate_for(request)?;
            state.pending_effects.remove(&receipt.operation_id);
            state.settled_effects.insert(receipt.operation_id.clone());
        }
        EventKind::EffectReconciled {
            lease_id,
            resolution,
        } => {
            require_root(&item.actor, root)?;
            let request = state
                .pending_effects
                .get(&resolution.operation_id)
                .ok_or(OrchestrationError::EffectDenied)?;
            if request.lease_id != *lease_id {
                return Err(OrchestrationError::EffectDenied);
            }
            resolution.validate_for(request)?;
            state.pending_effects.remove(&resolution.operation_id);
            state
                .settled_effects
                .insert(resolution.operation_id.clone());
        }
        _ => return Err(OrchestrationError::InvalidEvent),
    }
    Ok(())
}
