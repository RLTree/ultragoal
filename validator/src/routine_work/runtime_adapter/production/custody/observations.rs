use super::super::super::mediator::{
    IntentExecutionRequest, ObjectIdentity, RoutineMediationResult, RoutineNodeDisposition,
    StagedProgram,
};
use super::super::DurableSettlement;
use super::store::{TerminalMediation, TerminalNodeMediation};
use crate::routine_work::runtime_adapter::RoutineEffectIntent;
use crate::routine_work::{CleanupEvidence, ReservationFailureEvidence};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct IntentObservation {
    pub(super) intent_id: String,
    pub(super) node_id: String,
    pub(super) plan_order: usize,
    pub(super) program_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct OwnerObservation {
    pub(super) process_id: i32,
    pub(super) start_seconds: u64,
    pub(super) start_microseconds: u64,
    pub(super) nonce_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct ChildObservation {
    pub(super) process_id: i32,
    pub(super) process_group_id: i32,
    pub(super) executable_sha256: String,
    pub(super) executable_device: u64,
    pub(super) executable_inode: u64,
    pub(super) intent: IntentObservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct LaunchObservation {
    pub(super) directory: ObjectIdentity,
    pub(super) program: ObjectIdentity,
    pub(super) marker: ObjectIdentity,
    pub(super) seal: ObjectIdentity,
    pub(super) program_sha256: String,
    pub(super) intent: IntentObservation,
}

pub(in crate::routine_work::runtime_adapter::production::custody) struct TerminalObservation {
    pub(super) outcome: DurableSettlement,
    pub(super) result_sha256: String,
    pub(super) artifacts: BTreeMap<String, String>,
    pub(super) mediation: Option<TerminalMediation>,
    pub(super) process_cleanup: CleanupEvidence,
    pub(super) staged_cleanup: CleanupEvidence,
    pub(super) failure_evidence: Option<ReservationFailureEvidence>,
}

impl TerminalObservation {
    pub(super) fn mediated(
        outcome: DurableSettlement,
        result: &RoutineMediationResult,
        artifacts: BTreeMap<String, String>,
        process_cleanup: CleanupEvidence,
        staged_cleanup: CleanupEvidence,
    ) -> Result<Self, crate::routine_work::RoutineError> {
        let mediation = if outcome == DurableSettlement::Complete {
            Some(TerminalMediation {
                nodes: result
                    .nodes()
                    .iter()
                    .map(|node| {
                        if node.disposition != RoutineNodeDisposition::Executed {
                            return Err(crate::routine_work::RoutineError::new(
                                crate::routine_work::RoutineErrorId::InvalidRequest,
                                "routine-production-terminal-mediation-invalid",
                                None,
                            ));
                        }
                        Ok(TerminalNodeMediation {
                            intent_id: node.intent_id.clone(),
                            node_id: node.node_id.clone(),
                            plan_order: node.plan_order,
                            result_artifact_sha256: node
                                .result_artifact_sha256
                                .clone()
                                .ok_or_else(|| {
                                    crate::routine_work::RoutineError::new(
                                        crate::routine_work::RoutineErrorId::InvalidRequest,
                                        "routine-production-terminal-mediation-invalid",
                                        None,
                                    )
                                })?,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })
        } else {
            None
        };
        Ok(Self {
            outcome,
            result_sha256: crate::routine_work::digest::digest_of(result)?,
            artifacts,
            mediation,
            process_cleanup,
            staged_cleanup,
            failure_evidence: None,
        })
    }

    pub(super) fn failed(result_sha256: String, evidence: &ReservationFailureEvidence) -> Self {
        Self {
            outcome: DurableSettlement::Failed,
            result_sha256,
            artifacts: BTreeMap::new(),
            mediation: None,
            process_cleanup: evidence.process_cleanup.clone(),
            staged_cleanup: evidence.staged_cleanup.clone(),
            failure_evidence: Some(evidence.clone()),
        }
    }
}

pub(super) fn request_intent(intent: &RoutineEffectIntent) -> IntentObservation {
    IntentObservation {
        intent_id: intent.intent_id().to_owned(),
        node_id: intent.node_id().to_owned(),
        plan_order: intent.plan_order(),
        program_sha256: intent.program_sha256().to_owned(),
    }
}

fn observed_intent(intent: &IntentExecutionRequest, program_sha256: &str) -> IntentObservation {
    IntentObservation {
        intent_id: intent.intent_id().to_owned(),
        node_id: intent.node_id().to_owned(),
        plan_order: intent.plan_order(),
        program_sha256: program_sha256.to_owned(),
    }
}

pub(super) fn launch_observation(
    staged: &StagedProgram,
    intent: &IntentExecutionRequest,
) -> LaunchObservation {
    LaunchObservation {
        directory: staged.directory_identity,
        program: staged.executable.identity,
        marker: staged.marker_identity,
        seal: staged.seal_identity,
        program_sha256: staged.executable.sha256.clone(),
        intent: observed_intent(intent, &staged.executable.sha256),
    }
}

pub(super) fn child_observation(
    started: super::super::super::mediator::StartedProcessIdentity,
    executable: &super::super::super::mediator::PinnedExecutable,
    intent: &IntentExecutionRequest,
) -> ChildObservation {
    ChildObservation {
        process_id: started.process_id(),
        process_group_id: started.process_group_id(),
        executable_sha256: executable.sha256.clone(),
        executable_device: executable.identity.device,
        executable_inode: executable.identity.inode,
        intent: observed_intent(intent, &executable.sha256),
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
