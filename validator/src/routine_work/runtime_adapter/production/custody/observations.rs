use super::super::super::mediator::{IntentExecutionRequest, ObjectIdentity, StagedProgram};
use super::super::DurableSettlement;
use super::store::{AttemptState, IntentBinding, LaunchEntryIdentity, LaunchStageRecord};
use crate::routine_work::runtime_adapter::RoutineEffectIntent;
use crate::routine_work::{CleanupEvidence, ReservationFailureEvidence};

pub(super) fn request_intent(intent: &RoutineEffectIntent) -> IntentBinding {
    IntentBinding {
        intent_id: intent.intent_id().to_owned(),
        node_id: intent.node_id().to_owned(),
        plan_order: intent.plan_order(),
        program_sha256: intent.program_sha256().to_owned(),
    }
}

pub(super) fn observed_intent(
    intent: &IntentExecutionRequest,
    program_sha256: &str,
) -> IntentBinding {
    IntentBinding {
        intent_id: intent.intent_id().to_owned(),
        node_id: intent.node_id().to_owned(),
        plan_order: intent.plan_order(),
        program_sha256: program_sha256.to_owned(),
    }
}

pub(super) fn launch_record(
    staged: &StagedProgram,
    intent: &IntentExecutionRequest,
) -> LaunchStageRecord {
    LaunchStageRecord {
        directory: launch_identity(staged.directory_identity),
        program: launch_identity(staged.executable.identity),
        marker: launch_identity(staged.marker_identity),
        seal: launch_identity(staged.seal_identity),
        program_sha256: staged.executable.sha256.clone(),
        intent: observed_intent(intent, &staged.executable.sha256),
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

pub(super) fn cleanup_state(cleaned: bool) -> CleanupEvidence {
    if cleaned {
        CleanupEvidence::Succeeded
    } else {
        CleanupEvidence::NotRequired
    }
}

pub(super) fn cleanup_exact(evidence: &ReservationFailureEvidence) -> bool {
    let exact = |value: &CleanupEvidence| {
        matches!(
            value,
            CleanupEvidence::NotRequired | CleanupEvidence::Succeeded
        )
    };
    exact(&evidence.process_cleanup) && exact(&evidence.staged_cleanup)
}

fn launch_identity(identity: ObjectIdentity) -> LaunchEntryIdentity {
    LaunchEntryIdentity {
        device: identity.device,
        inode: identity.inode,
        mode: identity.mode,
        owner: identity.owner_user_id,
        links: identity.links,
        length: identity.length,
    }
}
