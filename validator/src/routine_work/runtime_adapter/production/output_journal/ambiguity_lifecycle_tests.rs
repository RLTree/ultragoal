use super::super::ledger::AttemptState;
use super::apply::{ApplyEvent, apply_observed};
use super::tests::{Fixture, reserve_attempt, scope};
use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const COMPONENTS: [&str; 3] = ["target", "target/routine", "target/routine/compile"];

#[test]
fn unchanged_unrecorded_stage_reconciles_then_fresh_and_repeat_apply_succeed() {
    for (ordinal, path) in COMPONENTS.into_iter().enumerate() {
        exercise_lifecycle(path, ordinal, false);
    }
}

#[test]
fn substituted_unrecorded_stage_stays_foreign_while_fresh_and_repeat_apply_succeed() {
    for (ordinal, path) in COMPONENTS.into_iter().enumerate() {
        exercise_lifecycle(path, ordinal, true);
    }
}

fn exercise_lifecycle(path: &str, ordinal: usize, substitute: bool) {
    let kind = if substitute {
        "substituted"
    } else {
        "unchanged"
    };
    let label = format!("{kind}-unrecorded-{ordinal}");
    let fixture = Fixture::new(&label);
    let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
    let first = reserve_attempt(
        &ledger,
        &label,
        observe(&fixture.workspace, &[scope()]).unwrap(),
        None,
        "initial",
    );
    let interrupted = apply_observed(&ledger, &first, &fixture.workspace, &mut |event| {
        if matches!(event, ApplyEvent::StageCreated(observed) if observed == path) {
            Err(error("routine-output-journal-test-interruption"))
        } else {
            Ok(())
        }
    });
    assert!(interrupted.is_err());
    let pending = ledger.pending_recovery(&first.binding).unwrap().unwrap();
    let component = pending
        .output_journal
        .components
        .iter()
        .find(|component| component.relative_path == path)
        .unwrap()
        .clone();
    assert!(component.staged.is_none());
    let stage = stage_path(&fixture, &component);
    let held = substitute.then(|| fixture.root.join(format!("held-stage-{ordinal}")));
    if let Some(held) = &held {
        fs::rename(&stage, held).unwrap();
        fs::create_dir(&stage).unwrap();
        fs::set_permissions(&stage, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let ambiguous = fs::metadata(&stage).unwrap();
    drop(ledger);

    let ledger = FileAuthorityLedger::open_existing(&fixture.authority).unwrap();
    let recovered = reserve_attempt(
        &ledger,
        &label,
        pending.output_journal,
        Some(pending.marker),
        "recovery",
    );
    let outcome = apply(&ledger, &recovered, &fixture.workspace).unwrap();
    let ApplyOutcome::UnrecordedStage(ambiguity) = &outcome else {
        panic!("unrecorded stage was not classified as ambiguous");
    };
    let ambiguity = ambiguity.clone();
    assert_eq!(ambiguity.relative_path, path);
    assert_eq!(
        ambiguity.creation_nonce,
        component.creation_nonce.as_deref().unwrap()
    );
    let resolution = super::resolve_application(&ledger, &recovered, outcome).unwrap_err();
    assert_eq!(
        resolution.cause(),
        "routine-production-output-ambiguity-reconciled-incomplete"
    );
    assert!(
        String::from_utf8(fs::read(fixture.authority.join("routine-authority.state")).unwrap())
            .unwrap()
            .contains("\"state\":\"incomplete\"")
    );
    ledger
        .reconcile_output_ambiguity(&recovered, &ambiguity)
        .unwrap();
    assert!(
        ledger
            .pending_recovery(&recovered.binding)
            .unwrap()
            .is_none()
    );
    assert_eq!(fs::metadata(&stage).unwrap().ino(), ambiguous.ino());
    assert!(!fixture.workspace.join(path).exists());

    let fresh_journal = observe(&fixture.workspace, &[scope()]).unwrap();
    let new_nonce = nonce_for(&fresh_journal, path).to_owned();
    assert_ne!(new_nonce, ambiguity.creation_nonce);
    let fresh = reserve_attempt(&ledger, &label, fresh_journal, None, "fresh");
    assert_applied(apply(&ledger, &fresh, &fixture.workspace).unwrap());
    assert!(fixture.workspace.join(path).is_dir());
    assert_eq!(fs::metadata(&stage).unwrap().ino(), ambiguous.ino());
    ledger
        .settle(&fresh, AttemptState::Failed, &BTreeMap::new())
        .unwrap();

    let repeat_journal = observe(&fixture.workspace, &[scope()]).unwrap();
    let repeat = reserve_attempt(&ledger, &label, repeat_journal, None, "repeat");
    assert_applied(apply(&ledger, &repeat, &fixture.workspace).unwrap());
    assert_eq!(fs::metadata(&stage).unwrap().ino(), ambiguous.ino());
    ledger
        .settle(&repeat, AttemptState::Failed, &BTreeMap::new())
        .unwrap();
    if let Some(held) = held {
        assert!(held.is_dir());
    }
    drop(ledger);
    fixture.teardown();
}

fn nonce_for<'a>(journal: &'a OutputProvisionJournal, path: &str) -> &'a str {
    journal
        .components
        .iter()
        .find(|component| component.relative_path == path)
        .and_then(|component| component.creation_nonce.as_deref())
        .unwrap()
}

fn stage_path(fixture: &Fixture, component: &OutputComponentJournal) -> std::path::PathBuf {
    let parent = component
        .relative_path
        .rsplit_once('/')
        .map_or("", |(parent, _)| parent);
    fixture.workspace.join(parent).join(format!(
        ".routine-output-{}",
        component.creation_nonce.as_deref().unwrap()
    ))
}

fn assert_applied(outcome: ApplyOutcome) {
    assert_eq!(outcome, ApplyOutcome::Applied);
}
