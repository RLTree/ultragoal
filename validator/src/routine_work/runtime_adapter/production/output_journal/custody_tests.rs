use super::super::ledger::AttemptState;
use super::apply::{ApplyEvent, apply_observed};
use super::tests::{Fixture, reserve, scope};
use super::*;
use std::os::unix::fs::symlink;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

#[derive(Clone, Copy, Debug)]
enum ForeignFinalKind {
    File,
    NonemptyDirectory,
    Symlink,
}

#[test]
fn foreign_empty_final_is_never_adopted_at_any_component() {
    for (ordinal, relative) in ["target", "target/routine", "target/routine/compile"]
        .into_iter()
        .enumerate()
    {
        let label = format!("foreign-empty-{ordinal}");
        let fixture = Fixture::new(&label);
        if let Some((parent, _)) = relative.rsplit_once('/') {
            fs::create_dir_all(fixture.workspace.join(parent)).unwrap();
        }
        let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
        let token = reserve(
            &ledger,
            &label,
            observe(&fixture.workspace, &[scope()]).unwrap(),
            None,
        );
        let foreign = fixture.workspace.join(relative);
        fs::create_dir(&foreign).unwrap();
        fs::set_permissions(&foreign, fs::Permissions::from_mode(0o700)).unwrap();
        let before = fs::metadata(&foreign).unwrap();
        let failure = apply(&ledger, &token, &fixture.workspace).unwrap_err();
        assert_eq!(
            failure.cause(),
            "routine-production-output-final-without-custody"
        );
        let after = fs::metadata(&foreign).unwrap();
        assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
        let pending = ledger.pending_recovery(&token.binding).unwrap().unwrap();
        assert!(
            pending
                .output_journal
                .components
                .iter()
                .find(|component| component.relative_path == relative)
                .is_some_and(|component| component.provisioned.is_none())
        );
        ledger
            .settle(&token, AttemptState::Failed, &BTreeMap::new())
            .unwrap();
        drop(ledger);
        fixture.teardown();
    }
}

#[test]
fn foreign_final_types_are_preserved_without_staging_or_adoption() {
    for kind in [
        ForeignFinalKind::File,
        ForeignFinalKind::NonemptyDirectory,
        ForeignFinalKind::Symlink,
    ] {
        let label = format!("foreign-final-{kind:?}");
        let fixture = Fixture::new(&label);
        let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
        let token = reserve(
            &ledger,
            &label,
            observe(&fixture.workspace, &[scope()]).unwrap(),
            None,
        );
        let final_path = fixture.workspace.join("target");
        match kind {
            ForeignFinalKind::File => fs::write(&final_path, b"foreign-file").unwrap(),
            ForeignFinalKind::NonemptyDirectory => {
                fs::create_dir(&final_path).unwrap();
                fs::write(final_path.join("foreign"), b"foreign-content").unwrap();
            }
            ForeignFinalKind::Symlink => symlink(&fixture.authority, &final_path).unwrap(),
        }
        let before = fs::symlink_metadata(&final_path).unwrap();
        let failure = apply(&ledger, &token, &fixture.workspace).unwrap_err();
        assert_eq!(
            failure.cause(),
            "routine-production-output-final-without-custody"
        );
        let after = fs::symlink_metadata(&final_path).unwrap();
        assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
        assert_eq!(
            fs::read_dir(&fixture.workspace)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".routine-output-"))
                .count(),
            0
        );
        drop(ledger);
        fixture.teardown();
    }
}

#[test]
fn reserved_creation_intent_recovers_before_the_first_workspace_mutation() {
    let fixture = Fixture::new("pre-creation-recovery");
    let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
    let first = reserve(
        &ledger,
        "pre-creation-recovery",
        observe(&fixture.workspace, &[scope()]).unwrap(),
        None,
    );
    assert!(fs::read_dir(&fixture.workspace).unwrap().next().is_none());
    let pending = ledger.pending_recovery(&first.binding).unwrap().unwrap();
    assert!(pending.output_journal.components.iter().all(|component| {
        component.creation_nonce.is_some()
            && component.staged.is_none()
            && component.provisioned.is_none()
    }));
    drop(ledger);

    let ledger = FileAuthorityLedger::open_existing(&fixture.authority).unwrap();
    let recovered = reserve(
        &ledger,
        "pre-creation-recovery",
        pending.output_journal,
        Some(pending.marker),
    );
    apply(&ledger, &recovered, &fixture.workspace).unwrap();
    assert!(fixture.workspace.join("target/routine/compile").is_dir());
    ledger
        .settle(&recovered, AttemptState::Failed, &BTreeMap::new())
        .unwrap();
    drop(ledger);
    fixture.teardown();
}

#[test]
fn exclusive_publish_preserves_a_concurrent_foreign_final() {
    let fixture = Fixture::new("publish-conflict");
    let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
    let token = reserve(
        &ledger,
        "publish-conflict",
        observe(&fixture.workspace, &[scope()]).unwrap(),
        None,
    );
    let failure = apply_observed(&ledger, &token, &fixture.workspace, &mut |event| {
        if matches!(event, ApplyEvent::StageRecorded("target")) {
            let foreign = fixture.workspace.join("target");
            fs::create_dir(&foreign).unwrap();
            fs::set_permissions(&foreign, fs::Permissions::from_mode(0o700)).unwrap();
        }
        Ok(())
    })
    .unwrap_err();
    assert_eq!(
        failure.cause(),
        "routine-production-output-publish-conflict"
    );
    let foreign = fs::metadata(fixture.workspace.join("target")).unwrap();
    let pending = ledger.pending_recovery(&token.binding).unwrap().unwrap();
    let staged = pending.output_journal.components[0].staged.unwrap();
    assert_ne!(
        (foreign.dev(), foreign.ino()),
        (staged.device, staged.inode)
    );
    assert!(stage_path(&fixture, &pending.output_journal.components[0]).is_dir());
    drop(ledger);
    fixture.teardown();
}

#[test]
fn substituted_staging_inode_is_preserved_and_never_published() {
    let fixture = Fixture::new("stage-substitution");
    let ledger = FileAuthorityLedger::open_or_initialize(&fixture.authority).unwrap();
    let token = reserve(
        &ledger,
        "stage-substitution",
        observe(&fixture.workspace, &[scope()]).unwrap(),
        None,
    );
    let interrupted = apply_observed(&ledger, &token, &fixture.workspace, &mut |event| {
        matches!(event, ApplyEvent::StageRecorded("target"))
            .then(|| Err(error("routine-output-journal-test-interruption")))
            .unwrap_or(Ok(()))
    });
    assert!(interrupted.is_err());
    let pending = ledger.pending_recovery(&token.binding).unwrap().unwrap();
    let component = &pending.output_journal.components[0];
    let stage = stage_path(&fixture, component);
    let held = fixture.workspace.join("held-stage");
    fs::rename(&stage, &held).unwrap();
    fs::create_dir(&stage).unwrap();
    fs::set_permissions(&stage, fs::Permissions::from_mode(0o700)).unwrap();
    let replacement = fs::metadata(&stage).unwrap();
    let recovered = reserve(
        &ledger,
        "stage-substitution",
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
    assert!(!fixture.workspace.join("target").exists());
    drop(ledger);
    fixture.teardown();
}

fn stage_path(fixture: &Fixture, component: &OutputComponentJournal) -> std::path::PathBuf {
    fixture.workspace.join(format!(
        ".routine-output-{}",
        component.creation_nonce.as_deref().unwrap()
    ))
}

pub(super) fn assert_no_staged_outputs(root: &Path) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap().map(Result::unwrap) {
            let name = entry.file_name().to_string_lossy().into_owned();
            assert!(
                !name.starts_with(".routine-output-"),
                "retained stage {name}"
            );
            if entry.file_type().unwrap().is_dir() {
                pending.push(entry.path());
            }
        }
    }
}
