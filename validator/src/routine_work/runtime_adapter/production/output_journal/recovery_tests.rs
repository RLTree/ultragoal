use super::super::ledger::AttemptState;
use super::apply::{ApplyEvent, apply_observed};
use super::tests::{Fixture, reserve, scope};
use super::*;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const COMPONENTS: [&str; 3] = ["target", "target/routine", "target/routine/compile"];

#[test]
fn recorded_stage_and_publish_boundaries_recover_exactly() {
    for (ordinal, (path, boundary)) in COMPONENTS
        .into_iter()
        .flat_map(|path| [1, 2, 3].map(move |boundary| (path, boundary)))
        .enumerate()
    {
        let label = format!("recorded-boundary-{ordinal}");
        let fixture = Fixture::new(&label);
        let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
        let first = reserve(
            &ledger,
            &label,
            observe(&fixture.workspace, &[scope()]).unwrap(),
            None,
        );
        let interrupted = apply_observed(&ledger, &first, &fixture.workspace, &mut |event| {
            if boundary_matches(event, path, boundary) {
                Err(error("routine-output-journal-test-interruption"))
            } else {
                Ok(())
            }
        });
        assert!(interrupted.is_err());
        let pending = ledger.pending_recovery(&first.binding).unwrap().unwrap();
        drop(ledger);

        let ledger = FileAuthorityLedger::open_existing(&fixture.authority).unwrap();
        let recovered = reserve(
            &ledger,
            &label,
            pending.output_journal,
            Some(pending.marker),
        );
        apply(&ledger, &recovered, &fixture.workspace).unwrap();
        assert!(fixture.workspace.join("target/routine/compile").is_dir());
        super::custody_tests::assert_no_staged_outputs(&fixture.workspace);
        ledger
            .settle(&recovered, AttemptState::Failed, &BTreeMap::new())
            .unwrap();
        assert!(
            ledger
                .pending_recovery(&recovered.binding)
                .unwrap()
                .is_none()
        );
        drop(ledger);
        fixture.teardown();
    }
}

#[test]
fn unrecorded_stage_replacement_is_never_adopted_at_any_component() {
    for (ordinal, path) in COMPONENTS.into_iter().enumerate() {
        let label = format!("unrecorded-stage-{ordinal}");
        let fixture = Fixture::new(&label);
        let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
        let first = reserve(
            &ledger,
            &label,
            observe(&fixture.workspace, &[scope()]).unwrap(),
            None,
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
            .unwrap();
        assert!(component.staged.is_none());
        let stage = stage_path(&fixture, component);
        let held = fixture
            .root
            .join(format!("held-unrecorded-stage-{ordinal}"));
        fs::rename(&stage, &held).unwrap();
        fs::create_dir(&stage).unwrap();
        fs::set_permissions(&stage, fs::Permissions::from_mode(0o700)).unwrap();
        let replacement = fs::metadata(&stage).unwrap();
        drop(ledger);

        let ledger = FileAuthorityLedger::open_existing(&fixture.authority).unwrap();
        let recovered = reserve(
            &ledger,
            &label,
            pending.output_journal,
            Some(pending.marker),
        );
        let failure = apply(&ledger, &recovered, &fixture.workspace).unwrap_err();
        assert_eq!(
            failure.cause(),
            "routine-production-output-stage-custody-changed"
        );
        assert_eq!(fs::metadata(&stage).unwrap().ino(), replacement.ino());
        assert!(held.is_dir());
        assert!(!fixture.workspace.join(path).exists());
        let still_pending = ledger
            .pending_recovery(&recovered.binding)
            .unwrap()
            .unwrap();
        assert!(
            still_pending
                .output_journal
                .components
                .iter()
                .find(|component| component.relative_path == path)
                .is_some_and(|component| component.staged.is_none())
        );
        drop(ledger);
        fixture.teardown();
    }
}

fn boundary_matches(event: ApplyEvent<'_>, path: &str, boundary: u8) -> bool {
    matches!(
        (event, boundary),
        (ApplyEvent::StageRecorded(observed), 1)
            | (ApplyEvent::Published(observed), 2)
            | (ApplyEvent::FinalRecorded(observed), 3)
            if observed == path
    )
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
