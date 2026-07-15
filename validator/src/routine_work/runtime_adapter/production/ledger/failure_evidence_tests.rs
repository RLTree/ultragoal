use super::*;
use crate::routine_work::{
    CleanupEvidence, FailureEvidence, RESERVATION_FAILURE_SCHEMA, ReservationFailureDisposition,
    ReservationFailureEvidence,
};

#[test]
fn exact_failure_evidence_survives_reopen_recovery_and_replay() {
    let mut root = TestRoot::new("failure-evidence-recovery");
    let ledger = FileAuthorityLedger::open_or_initialize(root.path()).unwrap();
    let token = reserve(&ledger, "failure-evidence");
    ledger.prepare_spawn(&token).unwrap();
    let first = evidence(
        &token,
        ReservationFailureDisposition::StartedPending,
        "first",
    );
    ledger.record_failure(&token, &first).unwrap();
    let recorded_state = root.state();
    ledger.record_failure(&token, &first).unwrap();
    assert_eq!(
        root.state(),
        recorded_state,
        "exact replay republished state"
    );
    assert_eq!(
        ledger.test_failure_records(&token.binding).unwrap(),
        [first.clone()]
    );
    let pending = ledger.pending_recovery(&token.binding).unwrap().unwrap();
    drop(ledger);

    let reopened = FileAuthorityLedger::open_existing(root.path()).unwrap();
    assert_eq!(
        reopened.test_failure_records(&token.binding).unwrap(),
        [first]
    );
    let recovered = reopened
        .reserve(ReservationSpec {
            binding: token.binding.clone(),
            request_id: token.request_id.clone(),
            grant_id: id("failure-evidence-recovery-grant"),
            recovery_marker: id("failure-evidence-recovery-marker"),
            recovery_for: Some(pending.marker),
            reuse_only: false,
            reuse_preauthorization: None,
            output_journal: pending.output_journal,
        })
        .unwrap();
    let second = evidence(
        &recovered,
        ReservationFailureDisposition::ReservedPending,
        "second",
    );
    reopened.record_failure(&recovered, &second).unwrap();
    assert_eq!(
        reopened.test_failure_records(&token.binding).unwrap(),
        [
            evidence(
                &token,
                ReservationFailureDisposition::StartedPending,
                "first"
            ),
            second.clone(),
        ]
    );

    let mut substituted = second;
    substituted.grant_id = id("failure-evidence-foreign-grant");
    let before_refusal = root.state();
    assert_eq!(
        reopened
            .record_failure(&recovered, &substituted)
            .unwrap_err()
            .cause(),
        "routine-production-failure-evidence-invalid"
    );
    assert_eq!(root.state(), before_refusal);
    reopened
        .settle(&recovered, AttemptState::Incomplete, &BTreeMap::new())
        .unwrap();
    assert!(reopened.pending_recovery(&token.binding).unwrap().is_none());
    let fresh = reopened
        .reserve(ReservationSpec {
            binding: token.binding.clone(),
            request_id: token.request_id.clone(),
            grant_id: id("failure-evidence-fresh-grant"),
            recovery_marker: id("failure-evidence-fresh-marker"),
            recovery_for: None,
            reuse_only: false,
            reuse_preauthorization: None,
            output_journal: token.output_journal.clone(),
        })
        .unwrap();
    assert_eq!(
        reopened.test_failure_records(&token.binding).unwrap().len(),
        2,
        "fresh retry erased prior cleanup evidence"
    );
    reopened
        .settle(&fresh, AttemptState::Failed, &BTreeMap::new())
        .unwrap();
    let repeat = reopened
        .reserve(ReservationSpec {
            binding: fresh.binding.clone(),
            request_id: fresh.request_id.clone(),
            grant_id: id("failure-evidence-repeat-grant"),
            recovery_marker: id("failure-evidence-repeat-marker"),
            recovery_for: None,
            reuse_only: false,
            reuse_preauthorization: None,
            output_journal: fresh.output_journal.clone(),
        })
        .unwrap();
    assert_eq!(
        reopened.test_failure_records(&token.binding).unwrap().len(),
        2
    );
    reopened
        .settle(&repeat, AttemptState::Failed, &BTreeMap::new())
        .unwrap();
    drop(reopened);
    root.teardown_after_assertions();
}

fn evidence(
    token: &ReservationToken,
    disposition: ReservationFailureDisposition,
    label: &str,
) -> ReservationFailureEvidence {
    ReservationFailureEvidence {
        schema_version: RESERVATION_FAILURE_SCHEMA.to_owned(),
        protocol_id: token.binding.protocol_id.clone(),
        grant_id: token.grant_id.clone(),
        recovery_marker: token.recovery_marker.clone(),
        primary: FailureEvidence::Error(error("failure-primary").evidence()),
        process_cleanup: CleanupEvidence::Error(error("process-cleanup").evidence()),
        staged_cleanup: CleanupEvidence::Error(error(label_cause(label)).evidence()),
        disposition,
    }
}

fn label_cause(label: &str) -> &'static str {
    match label {
        "first" => "staged-cleanup-first",
        "second" => "staged-cleanup-second",
        _ => "staged-cleanup-unknown",
    }
}
