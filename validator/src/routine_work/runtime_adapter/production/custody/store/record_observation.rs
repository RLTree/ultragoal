use super::*;
use crate::routine_work::runtime_adapter::production::DurableSettlement;
use crate::routine_work::runtime_adapter::production::custody::observations::{
    ChildObservation, IntentObservation, LaunchObservation, OwnerObservation,
};
use crate::routine_work::runtime_adapter::production::custody::transaction::ReservationSpec;
use std::cell::Cell;

pub(super) fn token(spec: &ReservationSpec) -> ReservationToken {
    ReservationToken {
        binding: spec.binding().clone(),
        owner: owner(spec.owner()),
        request_id: spec.request_id().to_owned(),
        grant_id: spec.grant_id().to_owned(),
        recovery_marker: spec.recovery_marker().to_owned(),
        expires_tick: Cell::new(0),
        output_journal: spec.output_journal().clone(),
        intents: spec.intents().iter().map(intent).collect(),
    }
}

fn intent(value: &IntentObservation) -> IntentBinding {
    IntentBinding {
        intent_id: value.intent_id.clone(),
        node_id: value.node_id.clone(),
        plan_order: value.plan_order,
        program_sha256: value.program_sha256.clone(),
    }
}

fn owner(value: &OwnerObservation) -> OwnerLease {
    OwnerLease {
        process_id: value.process_id,
        start_seconds: value.start_seconds,
        start_microseconds: value.start_microseconds,
        nonce_sha256: value.nonce_sha256.clone(),
    }
}

pub(super) fn child(value: &ChildObservation) -> ChildLease {
    ChildLease {
        process_id: value.process_id,
        process_group_id: value.process_group_id,
        executable_sha256: value.executable_sha256.clone(),
        executable_device: value.executable_device,
        executable_inode: value.executable_inode,
        intent: intent(&value.intent),
    }
}

pub(super) fn stage(value: &LaunchObservation) -> LaunchStageRecord {
    LaunchStageRecord {
        directory: launch_identity(value.directory),
        program: launch_identity(value.program),
        marker: launch_identity(value.marker),
        seal: launch_identity(value.seal),
        program_sha256: value.program_sha256.clone(),
        intent: intent(&value.intent),
    }
}

fn launch_identity(
    value: crate::routine_work::runtime_adapter::mediator::ObjectIdentity,
) -> LaunchEntryIdentity {
    LaunchEntryIdentity {
        device: value.device,
        inode: value.inode,
        mode: value.mode,
        owner: value.owner_user_id,
        links: value.links,
        length: value.length,
    }
}

pub(super) fn terminal_state(outcome: DurableSettlement) -> AttemptState {
    match outcome {
        DurableSettlement::Complete => AttemptState::Complete,
        DurableSettlement::Failed => AttemptState::Failed,
        DurableSettlement::Cancelled => AttemptState::Cancelled,
        DurableSettlement::Incomplete => AttemptState::Incomplete,
    }
}
