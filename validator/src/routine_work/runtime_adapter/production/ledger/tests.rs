use super::*;
use std::sync::{Arc, Barrier};

#[path = "fixture_ownership_tests.rs"]
mod fixture_ownership_tests;
#[path = "ledger_fixture_root.rs"]
mod ledger_fixture_root;
#[path = "reexecute_tests.rs"]
mod reexecute_tests;
#[path = "terminal_retry_tests.rs"]
mod terminal_retry_tests;

use ledger_fixture_root::TestRoot;

fn id(label: &str) -> String {
    sha256(format!("routine-ledger-test-{label}").as_bytes())
}

fn binding(label: &str) -> AuthorityBinding {
    AuthorityBinding {
        protocol_id: id(&format!("{label}-protocol")),
        effect_id: id(&format!("{label}-effect")),
        context_id: id(&format!("{label}-context")),
        candidate_id: id(&format!("{label}-candidate")),
        plan_id: id(&format!("{label}-plan")),
        snapshot_id: id(&format!("{label}-snapshot")),
    }
}

fn reserve(ledger: &FileAuthorityLedger, label: &str) -> ReservationToken {
    ledger.reserve(reservation(label)).unwrap()
}

fn reservation(label: &str) -> ReservationSpec {
    ReservationSpec {
        binding: binding(label),
        request_id: id(&format!("{label}-request")),
        grant_id: id(&format!("{label}-grant")),
        recovery_marker: id(&format!("{label}-recovery")),
        recovery_for: None,
        reuse_only: false,
        reuse_preauthorization: None,
    }
}

#[test]
fn exact_started_token_is_read_only_idempotent_for_later_batch_intents() {
    let mut root = TestRoot::new("idempotent-started");
    let ledger = FileAuthorityLedger::open_or_initialize(root.path()).unwrap();
    let token = reserve(&ledger, "batch");
    let reserved = root.state();

    ledger.prepare_spawn(&token).unwrap();
    let started = root.state();
    assert_ne!(started, reserved);

    ledger.prepare_spawn(&token).unwrap();
    assert_eq!(
        root.state(),
        started,
        "Started revalidation must not republish"
    );

    let artifacts = BTreeMap::from([(id("artifact"), id("witness"))]);
    ledger.stage_success(&token, &artifacts).unwrap();
    ledger
        .settle(&token, AttemptState::Complete, &artifacts)
        .unwrap();
    let terminal = root.state();
    let refused = ledger.prepare_spawn(&token).unwrap_err();
    assert_eq!(
        refused.cause(),
        "routine-production-spawn-authority-invalid"
    );
    assert_eq!(root.state(), terminal);
    drop(ledger);
    root.teardown_after_assertions();
}

#[test]
fn staged_publication_recovers_without_reauthorizing_artifact_bytes() {
    let mut root = TestRoot::new("staged-recovery");
    let ledger = FileAuthorityLedger::open_or_initialize(root.path()).unwrap();
    let token = reserve(&ledger, "staged-recovery");
    let artifacts = BTreeMap::from([(id("staged-artifact"), id("staged-witness"))]);
    ledger.prepare_spawn(&token).unwrap();
    ledger.stage_success(&token, &artifacts).unwrap();

    let pending = ledger.pending_recovery(&token.binding).unwrap().unwrap();
    let recovered = ledger
        .reserve(ReservationSpec {
            binding: token.binding.clone(),
            request_id: token.request_id.clone(),
            grant_id: id("staged-recovery-grant-two"),
            recovery_marker: id("staged-recovery-marker-two"),
            recovery_for: Some(pending.marker),
            reuse_only: false,
            reuse_preauthorization: None,
        })
        .unwrap();
    assert!(
        ledger
            .authenticates(&recovered, &id("staged-artifact"), &id("staged-witness"))
            .unwrap()
    );
    ledger.stage_success(&recovered, &artifacts).unwrap();
    ledger
        .settle(&recovered, AttemptState::Complete, &artifacts)
        .unwrap();
    assert!(ledger.pending_recovery(&token.binding).unwrap().is_none());
    drop(ledger);
    root.teardown_after_assertions();
}

#[test]
fn candidate_source_command_and_cross_attempt_substitution_never_prepare() {
    let mut root = TestRoot::new("substitution");
    let ledger = FileAuthorityLedger::open_or_initialize(root.path()).unwrap();
    let token = reserve(&ledger, "original");
    let reserved = root.state();

    let mut candidate = token.clone();
    candidate.binding.candidate_id = id("substituted-candidate");
    let mut source = token.clone();
    source.binding.context_id = id("substituted-source-context");
    let mut command = token.clone();
    command.binding.plan_id = id("substituted-command-plan");
    command.binding.effect_id = id("substituted-command-effect");
    let mut request = token.clone();
    request.request_id = id("substituted-request");
    let mut grant = token.clone();
    grant.grant_id = id("cross-attempt-grant");

    for substituted in [&candidate, &source, &command, &request, &grant] {
        let refused = ledger.prepare_spawn(substituted).unwrap_err();
        assert!(
            matches!(
                refused.cause(),
                "routine-production-reservation-binding-invalid"
                    | "routine-production-reservation-missing"
            ),
            "unexpected false-pass cause: {refused}"
        );
        assert_eq!(root.state(), reserved);
    }

    let other = reserve(&ledger, "other-attempt");
    let after_other_reservation = root.state();
    let mut crossed = token.clone();
    crossed.binding = other.binding.clone();
    let refused = ledger.prepare_spawn(&crossed).unwrap_err();
    assert_eq!(
        refused.cause(),
        "routine-production-reservation-binding-invalid"
    );
    assert_eq!(root.state(), after_other_reservation);

    ledger.prepare_spawn(&token).unwrap();
    drop(ledger);
    root.teardown_after_assertions();
}

#[test]
fn expired_and_concurrently_conflicting_preparations_fail_closed() {
    let mut expired_root = TestRoot::new("expired");
    let expired_ledger = FileAuthorityLedger::open_or_initialize(expired_root.path()).unwrap();
    let mut expired = reserve(&expired_ledger, "expired");
    expired_ledger.prepare_spawn(&expired).unwrap();
    expired_ledger
        .test_expire_reservation(&mut expired)
        .unwrap();
    let expired_state = expired_root.state();
    let refused = expired_ledger.prepare_spawn(&expired).unwrap_err();
    assert_eq!(
        refused.cause(),
        "routine-production-spawn-authority-invalid"
    );
    assert_eq!(expired_root.state(), expired_state);

    let mut race_root = TestRoot::new("conflicting-race");
    let ledger = Arc::new(FileAuthorityLedger::open_or_initialize(race_root.path()).unwrap());
    let valid = reserve(&ledger, "race");
    let exact = valid.clone();
    let mut conflicting = valid.clone();
    conflicting.binding.candidate_id = id("race-conflicting-candidate");
    let barrier = Arc::new(Barrier::new(3));

    let valid_ledger = Arc::clone(&ledger);
    let valid_barrier = Arc::clone(&barrier);
    let valid_thread = std::thread::spawn(move || {
        valid_barrier.wait();
        valid_ledger.prepare_spawn(&valid)
    });
    let conflict_ledger = Arc::clone(&ledger);
    let conflict_barrier = Arc::clone(&barrier);
    let conflict_thread = std::thread::spawn(move || {
        conflict_barrier.wait();
        conflict_ledger.prepare_spawn(&conflicting)
    });
    barrier.wait();

    valid_thread.join().unwrap().unwrap();
    let conflict = conflict_thread.join().unwrap().unwrap_err();
    assert_eq!(
        conflict.cause(),
        "routine-production-reservation-binding-invalid"
    );
    let started = race_root.state();
    ledger.prepare_spawn(&exact).unwrap();
    assert_eq!(race_root.state(), started);
    drop(ledger);
    drop(expired_ledger);
    race_root.teardown_after_assertions();
    expired_root.teardown_after_assertions();
}
