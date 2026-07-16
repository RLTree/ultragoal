use super::super::super::execution_authority::RequestSeal;
use super::super::terminal_settlement_fixture::{TerminalDurable, attempt_grant, staged_fixture};
pub(super) use super::super::terminal_settlement_fixture::{
    finish_recovery, grant_recovery_marker, retain_stage,
};
use super::super::*;
use std::sync::atomic::Ordering;

#[derive(Clone, Copy)]
pub(super) enum CleanupCase {
    Success,
    Error,
    Panic,
}

pub(super) struct ProducerAttempt {
    pub(super) grant: RoutineRootGrant,
    pub(super) durable: Arc<TerminalDurable>,
    pub(super) stage_root: std::path::PathBuf,
    pub(super) staged: StagedProgram,
}

pub(super) fn producer_attempt(label: &str, cleanup: CleanupCase) -> ProducerAttempt {
    let durable = Arc::new(TerminalDurable::default());
    match cleanup {
        CleanupCase::Success => {}
        CleanupCase::Error => {
            *durable
                .cleanup_failure
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                Some("producer-staged-cleanup-error");
        }
        CleanupCase::Panic => {
            *durable
                .cleanup_panic
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                Some("producer-staged-cleanup-panic");
        }
    }
    let grant = attempt_grant(label, Some(durable.clone()), None);
    let (stage_root, staged) = staged_fixture(label);
    ProducerAttempt {
        grant,
        durable,
        stage_root,
        staged,
    }
}

pub(super) fn mediated_token(plan_order: usize, next_order: usize) -> RoutineMediatedIntent {
    let seal = Arc::new(RequestSeal::new(1, "producer-transition-seal".to_owned()));
    seal.begin_mediation().unwrap();
    seal.next_order.store(next_order, Ordering::Release);
    RoutineMediatedIntent::new(
        "producer-request".to_owned(),
        "producer-protocol".to_owned(),
        intent(plan_order),
        seal,
    )
}

fn intent(plan_order: usize) -> RoutineEffectIntent {
    RoutineEffectIntent {
        protocol_id: "producer-protocol".to_owned(),
        intent_id: "producer-intent".to_owned(),
        plan_order,
        node_id: "producer-node".to_owned(),
        behavior_id: "rust-source-syntax-v1".to_owned(),
        selected_tool: "ultragoal-current-program".to_owned(),
        tool_identity_sha256: "unused".to_owned(),
        program_path_hex: "unused".to_owned(),
        program_sha256: "unused".to_owned(),
        program_byte_length: 0,
        program_unix_mode: None,
        argv: Vec::new(),
        working_directory: ".".to_owned(),
        environment_policy: "clear-all-allowlisted-v1",
        environment_keys: Vec::new(),
        environment_sha256: "unused".to_owned(),
        environment: BTreeMap::new(),
        read_authority_policy: "default-deny-exact-bound-read-v1",
        read_source_paths: Vec::new(),
        read_authority_sha256: "unused".to_owned(),
        read_sources: Vec::new(),
        mediation_preflight: "revalidate-before-effect-v1",
        timeout_ms: 1,
        output_budget_bytes: 1,
        declared_output_scopes: Vec::new(),
        expected_dependency_nodes: Vec::new(),
        input_id: "producer-input".to_owned(),
    }
}

pub(super) fn recorded(durable: &TerminalDurable) -> ReservationFailureEvidence {
    let records = durable
        .failure_records
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(records.len(), 1);
    records[0].clone()
}

pub(super) fn expected_cleanup(case: CleanupCase) -> CleanupEvidence {
    match case {
        CleanupCase::Success => CleanupEvidence::Succeeded,
        CleanupCase::Error => {
            CleanupEvidence::Error(mediator_error("producer-staged-cleanup-error").evidence())
        }
        CleanupCase::Panic => CleanupEvidence::Panic(PanicEvidence::capture(
            &"producer-staged-cleanup-panic".to_owned(),
        )),
    }
}

pub(super) fn assert_error(evidence: &FailureEvidence, cause: &str) {
    match evidence {
        FailureEvidence::Error(error) => assert_eq!(error.cause, cause),
        other => panic!("expected error evidence, got {other:?}"),
    }
}
