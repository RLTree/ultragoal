use super::super::ledger::AttemptState;
use super::apply::{ApplyEvent, apply_observed};
use super::tests::{Fixture, reservation_attempt, reserve_attempt, scope};
use super::*;
use std::sync::{Arc, Barrier};

#[test]
fn initial_ambiguity_cannot_terminalize_without_recovery_authority() {
    let fixture = Fixture::new("initial-authority");
    let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
    let first = reserve_attempt(
        &ledger,
        "initial-authority",
        observe(&fixture.workspace, &[scope()]).unwrap(),
        None,
        "initial",
    );
    let interrupted = apply_observed(&ledger, &first, &fixture.workspace, &mut |event| {
        matches!(event, ApplyEvent::StageCreated("target"))
            .then(|| Err(error("routine-output-journal-test-interruption")))
            .unwrap_or(Ok(()))
    });
    assert!(interrupted.is_err());
    let before = state(&fixture);
    let pending = ledger.pending_recovery(&first.binding).unwrap().unwrap();
    let outcome = apply(&ledger, &first, &fixture.workspace).unwrap();
    let refusal =
        super::super::production_issuance::resolve_output_application(&ledger, &first, outcome)
            .unwrap_err();
    assert_eq!(
        refusal.cause(),
        "routine-production-output-ambiguity-recovery-required"
    );
    assert_eq!(state(&fixture), before);
    assert_eq!(
        ledger
            .pending_recovery(&first.binding)
            .unwrap()
            .unwrap()
            .marker,
        pending.marker
    );
    drop(ledger);
    fixture.teardown();
}

#[test]
fn substituted_recovery_authority_cannot_terminalize_or_erase_an_attempt() {
    let (fixture, ledger, recovered, ambiguity, stage) = interrupted_recovery("authority");
    let before = state(&fixture);
    let mut wrong_grant = recovered.clone();
    wrong_grant.grant_id = "0".repeat(64);
    let mut wrong_marker = recovered.clone();
    wrong_marker.recovery_for = Some("1".repeat(64));
    let mut wrong_protocol = recovered.clone();
    wrong_protocol.binding.protocol_id = "2".repeat(64);
    let mut wrong_path = ambiguity.clone();
    wrong_path.relative_path = "target/routine/other".to_owned();
    let mut wrong_nonce = ambiguity.clone();
    wrong_nonce.creation_nonce = "3".repeat(64);

    for (token, observed) in [
        (&wrong_grant, &ambiguity),
        (&wrong_marker, &ambiguity),
        (&wrong_protocol, &ambiguity),
        (&recovered, &wrong_path),
        (&recovered, &wrong_nonce),
    ] {
        assert!(ledger.reconcile_output_ambiguity(token, observed).is_err());
        assert_eq!(state(&fixture), before);
    }

    ledger
        .reconcile_output_ambiguity(&recovered, &ambiguity)
        .unwrap();
    let terminal = state(&fixture);
    ledger
        .reconcile_output_ambiguity(&recovered, &ambiguity)
        .unwrap();
    assert_eq!(state(&fixture), terminal);
    assert!(
        ledger
            .pending_recovery(&recovered.binding)
            .unwrap()
            .is_none()
    );
    let fresh_journal = observe(&fixture.workspace, &[scope()]).unwrap();
    let fresh = reserve_attempt(&ledger, "authority", fresh_journal, None, "fresh");
    let fresh_pending = ledger.pending_recovery(&fresh.binding).unwrap().unwrap();
    assert!(
        ledger
            .reconcile_output_ambiguity(&recovered, &ambiguity)
            .is_err()
    );
    assert_eq!(
        ledger
            .pending_recovery(&fresh.binding)
            .unwrap()
            .unwrap()
            .marker,
        fresh_pending.marker
    );
    assert!(stage.is_dir());
    ledger
        .settle(&fresh, AttemptState::Failed, &BTreeMap::new())
        .unwrap();
    drop(ledger);
    fixture.teardown();
}

#[test]
fn concurrent_reconciliation_and_retry_have_one_terminal_and_one_publisher() {
    let (fixture, ledger, recovered, ambiguity, stage) = interrupted_recovery("concurrent");
    let ledger = Arc::new(ledger);
    let barrier = Arc::new(Barrier::new(3));
    let mut reconciliations = Vec::new();
    for _ in 0..2 {
        let ledger = Arc::clone(&ledger);
        let barrier = Arc::clone(&barrier);
        let token = recovered.clone();
        let ambiguity = ambiguity.clone();
        reconciliations.push(std::thread::spawn(move || {
            barrier.wait();
            ledger.reconcile_output_ambiguity(&token, &ambiguity)
        }));
    }
    barrier.wait();
    for reconciliation in reconciliations {
        reconciliation.join().unwrap().unwrap();
    }
    assert!(
        ledger
            .pending_recovery(&recovered.binding)
            .unwrap()
            .is_none()
    );

    let journal = observe(&fixture.workspace, &[scope()]).unwrap();
    let left = reservation_attempt("concurrent", journal.clone(), None, "left");
    let right = reservation_attempt("concurrent", journal, None, "right");
    let barrier = Arc::new(Barrier::new(3));
    let left_ledger = Arc::clone(&ledger);
    let left_barrier = Arc::clone(&barrier);
    let left = std::thread::spawn(move || {
        left_barrier.wait();
        left_ledger.reserve(left)
    });
    let right_ledger = Arc::clone(&ledger);
    let right_barrier = Arc::clone(&barrier);
    let right = std::thread::spawn(move || {
        right_barrier.wait();
        right_ledger.reserve(right)
    });
    barrier.wait();
    let winner = match (left.join().unwrap(), right.join().unwrap()) {
        (Ok(token), Err(refusal)) | (Err(refusal), Ok(token)) => {
            assert_eq!(
                refusal.cause(),
                "routine-production-semantic-effect-replayed"
            );
            token
        }
        _ => panic!("concurrent retry did not produce exactly one reservation"),
    };
    assert_eq!(
        apply(&ledger, &winner, &fixture.workspace).unwrap(),
        ApplyOutcome::Applied
    );
    assert!(fixture.workspace.join("target/routine/compile").is_dir());
    assert!(stage.is_dir());
    ledger
        .settle(&winner, AttemptState::Failed, &BTreeMap::new())
        .unwrap();
    drop(ledger);
    fixture.teardown();
}

fn interrupted_recovery(
    label: &str,
) -> (
    Fixture,
    FileAuthorityLedger,
    ReservationToken,
    OutputStageAmbiguity,
    std::path::PathBuf,
) {
    let fixture = Fixture::new(label);
    let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
    let first = reserve_attempt(
        &ledger,
        label,
        observe(&fixture.workspace, &[scope()]).unwrap(),
        None,
        "initial",
    );
    let interrupted = apply_observed(&ledger, &first, &fixture.workspace, &mut |event| {
        matches!(event, ApplyEvent::StageCreated("target"))
            .then(|| Err(error("routine-output-journal-test-interruption")))
            .unwrap_or(Ok(()))
    });
    assert!(interrupted.is_err());
    let pending = ledger.pending_recovery(&first.binding).unwrap().unwrap();
    let component = &pending.output_journal.components[0];
    let stage = fixture.workspace.join(format!(
        ".routine-output-{}",
        component.creation_nonce.as_deref().unwrap()
    ));
    let recovered = reserve_attempt(
        &ledger,
        label,
        pending.output_journal,
        Some(pending.marker),
        "recovery",
    );
    let ApplyOutcome::UnrecordedStage(ambiguity) =
        apply(&ledger, &recovered, &fixture.workspace).unwrap()
    else {
        panic!("unrecorded stage was not classified as ambiguous");
    };
    (fixture, ledger, recovered, ambiguity, stage)
}

fn state(fixture: &Fixture) -> Vec<u8> {
    fs::read(fixture.authority.join("routine-authority.state")).unwrap()
}
