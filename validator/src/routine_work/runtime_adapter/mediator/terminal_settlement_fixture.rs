use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);
static NEXT_RESERVATION: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
pub(super) struct TerminalDurable {
    pub(super) settlements: Mutex<Vec<DurableSettlement>>,
    pub(super) failure_records: Mutex<Vec<ReservationFailureEvidence>>,
    pub(super) failure_record_error: Mutex<Option<&'static str>>,
    pub(super) failure_record_panic: Mutex<Option<&'static str>>,
    pub(super) cleanup_failure: Mutex<Option<&'static str>>,
    pub(super) cleanup_panic: Mutex<Option<&'static str>>,
    pub(super) cleanup_calls: AtomicUsize,
    staged_program: Mutex<Option<StagedProgram>>,
}

impl DurableAttemptAuthority for TerminalDurable {
    fn validate_reserved(&self) -> Result<(), RoutineError> {
        Ok(())
    }

    fn stage_program(&self, _program: &PinnedExecutable) -> Result<StagedProgram, RoutineError> {
        self.staged_program
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
            .ok_or_else(|| mediator_error("terminal-settlement-test-stage-unavailable"))
    }

    fn cleanup_staged(&self, _staged: &StagedProgram) -> Result<(), RoutineError> {
        self.cleanup_calls.fetch_add(1, Ordering::SeqCst);
        let cleanup_panic = *self
            .cleanup_panic
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(payload) = cleanup_panic {
            panic!("{payload}");
        }
        match *self
            .cleanup_failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            Some(cause) => Err(mediator_error(cause)),
            None => Ok(()),
        }
    }

    fn prepare_spawn(&self) -> Result<(), RoutineError> {
        Ok(())
    }

    fn stage_success(&self, _artifacts: &BTreeMap<String, String>) -> Result<(), RoutineError> {
        Ok(())
    }

    fn record_failure(&self, evidence: &ReservationFailureEvidence) -> Result<(), RoutineError> {
        if let Some(payload) = *self
            .failure_record_panic
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            panic!("{payload}");
        }
        if let Some(cause) = *self
            .failure_record_error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            return Err(mediator_error(cause));
        }
        self.failure_records
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(evidence.clone());
        Ok(())
    }

    fn settle(
        &self,
        outcome: DurableSettlement,
        _artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        self.settlements
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(outcome);
        Ok(())
    }

    fn authenticates_artifact(&self, _digest: &str, _witness: &str) -> Result<bool, RoutineError> {
        Ok(false)
    }

    fn recovery_is_durable(&self) -> bool {
        true
    }

    fn reuse_only(&self) -> bool {
        false
    }
}

pub(super) fn attempt(
    label: &str,
    durable: Option<Arc<dyn DurableAttemptAuthority>>,
    started: bool,
    prior_recovery_marker: Option<String>,
) -> AttemptReservation {
    let grant = reservation_grant(
        label,
        &format!("terminal-protocol-{label}"),
        prior_recovery_marker,
        durable,
    );
    let reservation = reserve_grant(&grant).unwrap();
    if started {
        reservation.mark_started().unwrap();
    }
    reservation
}

pub(super) fn recovering_attempt(
    label: &str,
    protocol_id: &str,
    recovery_for: String,
    durable: Option<Arc<dyn DurableAttemptAuthority>>,
    started: bool,
) -> AttemptReservation {
    let grant = reservation_grant(label, protocol_id, Some(recovery_for), durable);
    let reservation = reserve_grant(&grant).unwrap();
    if started {
        reservation.mark_started().unwrap();
    }
    reservation
}

pub(super) fn pending_recovery(label: &str) -> (String, String) {
    let reservation = attempt(label, None, true, None);
    let protocol = reservation.protocol_id().clone();
    let marker = reservation.recovery_marker().clone();
    run_reserved(reservation, |_| {
        Err::<(), _>(mediator_error("terminal-fixture-pending-recovery"))
    })
    .unwrap_err();
    (protocol, marker)
}

pub(super) fn recovery_grant(
    protocol_id: &str,
    recovery_for: Option<String>,
    durable: Arc<dyn DurableAttemptAuthority>,
) -> RoutineRootGrant {
    reservation_grant("recovery", protocol_id, recovery_for, Some(durable))
}

pub(super) fn retry_grant(
    reservation: &AttemptReservation,
    durable: Arc<dyn DurableAttemptAuthority>,
) -> RoutineRootGrant {
    reservation_grant("retry", reservation.protocol_id(), None, Some(durable))
}

fn reservation_grant(
    label: &str,
    protocol_id: &str,
    recovery_for: Option<String>,
    durable: Option<Arc<dyn DurableAttemptAuthority>>,
) -> RoutineRootGrant {
    let ordinal = NEXT_RESERVATION.fetch_add(1, Ordering::Relaxed);
    RoutineRootGrant {
        grant_id: format!("terminal-grant-{label}-{ordinal}"),
        session_id: format!("terminal-session-{ordinal}"),
        request_id: format!("terminal-request-{ordinal}"),
        protocol_id: protocol_id.to_owned(),
        context_id: "terminal-context".to_owned(),
        candidate_id: "terminal-candidate".to_owned(),
        plan_id: "terminal-plan".to_owned(),
        snapshot_id: "terminal-snapshot".to_owned(),
        allowed_output_scopes: Vec::new(),
        recovery_for,
        seal: format!("terminal-seal-{ordinal}"),
        durable,
    }
}

pub(super) fn staged_fixture(label: &str) -> (PathBuf, StagedProgram) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let parent = manifest
        .parent()
        .expect("reservation fixture manifest has no workspace parent")
        .join("target/routine-reservation-lifecycle-fixtures");
    fs::create_dir_all(&parent).expect("reservation fixture parent is unavailable");
    let directory = parent.join(format!(
        "{label}-{}-{}",
        std::process::id(),
        NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory).unwrap();
    let program = directory.join("program");
    let marker = directory.join("authority");
    let seal = directory.join("seal");
    fs::copy("/usr/bin/true", &program).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o555)).unwrap();
    let marker_bytes = b"reservation-unwind-marker\n".to_vec();
    let seal_bytes = b"reservation-unwind-seal\n".to_vec();
    fs::write(&marker, &marker_bytes).unwrap();
    fs::write(&seal, &seal_bytes).unwrap();
    let executable = PinnedExecutable::open_unbound(&program).unwrap();
    let staged = StagedProgram {
        executable,
        directory: directory.clone(),
        marker: marker.clone(),
        seal: seal.clone(),
        marker_bytes,
        seal_bytes,
        directory_identity: ObjectIdentity::from(&fs::metadata(&directory).unwrap()),
        marker_identity: ObjectIdentity::from(&fs::metadata(marker).unwrap()),
        seal_identity: ObjectIdentity::from(&fs::metadata(seal).unwrap()),
    };
    (directory, staged)
}

pub(super) fn retain_stage(
    attempt: &AttemptReservation,
    durable: &TerminalDurable,
    staged: StagedProgram,
) {
    *durable
        .staged_program
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(staged);
    let source = PinnedExecutable::open_unbound(std::path::Path::new("/usr/bin/true")).unwrap();
    attempt.stage_and_use(&source, |_| Ok(())).unwrap();
}
