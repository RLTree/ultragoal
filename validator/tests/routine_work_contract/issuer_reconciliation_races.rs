use std::ffi::CString;
use std::fs;
use std::os::unix::fs::{FileTypeExt, symlink};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::owned_compile_quarantine::{CleanupDirective, CleanupOutcome, CleanupStage};
use super::owned_compile_scratch::OwnedCompileScratch;

#[test]
fn identity_to_rename_symlink_swap_never_reports_foreign_reconciled() {
    let mut fixture = displaced_fixture("routine-issuer-reconcile-symlink");
    let quarantine = fixture.owned.recovery_path().unwrap();
    let transplant = quarantine.with_extension("transplanted");
    let symlink_target = fixture.held.clone();
    let outcome = fixture.owned.recover_with(|stage| {
        if stage == CleanupStage::ForeignIdentityValidated {
            fs::rename(&quarantine, &transplant).unwrap();
            symlink(&symlink_target, &quarantine).unwrap();
        }
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::AmbiguousPartialEffect);
    assert!(
        fs::symlink_metadata(&fixture.original)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(tree_digest(&transplant), fixture.foreign_digest);
    assert_eq!(tree_digest(&fixture.held), fixture.genuine_digest);
    assert!(fixture.owned.ambiguous_destination().unwrap().is_some());
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::AmbiguousPartialEffect
    );

    fs::remove_file(&fixture.original).unwrap();
    fs::rename(&transplant, &quarantine).unwrap();
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::ReconciledForeign
    );
    assert_eq!(tree_digest(&fixture.original), fixture.foreign_digest);
    finish_fixture(fixture, &quarantine, &transplant);
}

#[test]
fn rename_to_postcheck_fifo_swap_is_typed_ambiguous_and_recoverable() {
    let mut fixture = displaced_fixture("routine-issuer-reconcile-fifo");
    let quarantine = fixture.owned.recovery_path().unwrap();
    let transplant = quarantine.with_extension("transplanted");
    let original = fixture.original.clone();
    let outcome = fixture.owned.recover_with(|stage| {
        if stage == CleanupStage::ForeignMoved {
            fs::rename(&original, &transplant).unwrap();
            let name = CString::new(original.as_os_str().as_encoded_bytes()).unwrap();
            assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        }
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::AmbiguousPartialEffect);
    assert!(
        fs::symlink_metadata(&fixture.original)
            .unwrap()
            .file_type()
            .is_fifo()
    );
    assert_eq!(tree_digest(&transplant), fixture.foreign_digest);
    assert_eq!(tree_digest(&fixture.held), fixture.genuine_digest);
    assert!(fixture.owned.ambiguous_destination().unwrap().is_some());
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::AmbiguousPartialEffect
    );

    fs::remove_file(&fixture.original).unwrap();
    fs::rename(&transplant, &quarantine).unwrap();
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::ReconciledForeign
    );
    assert_eq!(tree_digest(&fixture.original), fixture.foreign_digest);
    finish_fixture(fixture, &quarantine, &transplant);
}

#[test]
fn postmove_quarantine_refill_blocks_reconciled_outcome() {
    let mut fixture = displaced_fixture("routine-issuer-reconcile-refill");
    let quarantine = fixture.owned.recovery_path().unwrap();
    let outcome = fixture.owned.recover_with(|stage| {
        if stage == CleanupStage::ForeignMoved {
            fs::create_dir(&quarantine).unwrap();
            fs::write(quarantine.join("unrelated"), b"unrelated bytes\n").unwrap();
        }
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::AmbiguousPartialEffect);
    assert_eq!(tree_digest(&fixture.original), fixture.foreign_digest);
    assert_eq!(
        fs::read(quarantine.join("unrelated")).unwrap(),
        b"unrelated bytes\n"
    );
    assert_eq!(tree_digest(&fixture.held), fixture.genuine_digest);
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::AmbiguousPartialEffect
    );
    assert!(quarantine.exists());

    fs::remove_dir_all(&quarantine).unwrap();
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::ReconciledForeign
    );
    finish_fixture(
        fixture,
        &quarantine,
        &quarantine.with_extension("transplanted"),
    );
}

#[test]
fn quarantine_disappearance_between_check_and_rename_retains_custody() {
    let mut fixture = displaced_fixture("routine-issuer-reconcile-disappearance");
    let quarantine = fixture.owned.recovery_path().unwrap();
    let transplant = quarantine.with_extension("transplanted");
    let outcome = fixture.owned.recover_with(|stage| {
        if stage == CleanupStage::ForeignIdentityValidated {
            fs::rename(&quarantine, &transplant).unwrap();
        }
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::AmbiguousPartialEffect);
    assert!(!quarantine.exists());
    assert_eq!(tree_digest(&transplant), fixture.foreign_digest);
    assert_eq!(
        fixture.owned.recovery_path().as_deref(),
        Some(quarantine.as_path())
    );
    fs::rename(&transplant, &quarantine).unwrap();
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::ReconciledForeign
    );
    finish_fixture(fixture, &quarantine, &transplant);
}

struct DisplacedFixture {
    owned: OwnedCompileScratch,
    foreign: OwnedCompileScratch,
    original: PathBuf,
    held: PathBuf,
    genuine_digest: [u8; 32],
    foreign_digest: [u8; 32],
}

fn displaced_fixture(label: &str) -> DisplacedFixture {
    let mut owned = OwnedCompileScratch::claim(&format!("{label}-owner"));
    let foreign = OwnedCompileScratch::claim(&format!("{label}-foreign"));
    let original = owned.path().to_path_buf();
    let foreign_original = foreign.path().to_path_buf();
    let held = original.with_file_name(format!(
        "{}-held",
        original.file_name().unwrap().to_string_lossy()
    ));
    fs::write(original.join("genuine"), b"genuine bytes\n").unwrap();
    fs::write(foreign_original.join("foreign"), b"foreign bytes\n").unwrap();
    let genuine_digest = tree_digest(&original);
    let foreign_digest = tree_digest(&foreign_original);
    let outcome = owned.recover_with(|stage| match stage {
        CleanupStage::Authenticated => {
            fs::rename(&original, &held).unwrap();
            fs::rename(&foreign_original, &original).unwrap();
            CleanupDirective::Continue
        }
        CleanupStage::ForeignQuarantined => CleanupDirective::Interrupt,
        _ => CleanupDirective::Continue,
    });
    assert_eq!(outcome, CleanupOutcome::AmbiguousPartialEffect);
    DisplacedFixture {
        owned,
        foreign,
        original,
        held,
        genuine_digest,
        foreign_digest,
    }
}

fn finish_fixture(mut fixture: DisplacedFixture, quarantine: &Path, transplant: &Path) {
    assert!(!quarantine.exists());
    assert!(!transplant.exists());
    assert_eq!(tree_digest(&fixture.original), fixture.foreign_digest);
    assert_eq!(tree_digest(&fixture.held), fixture.genuine_digest);
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::RefusedZeroWrite
    );
    assert_eq!(
        fixture.foreign.recover_interrupted(),
        CleanupOutcome::RefusedZeroWrite
    );
    fs::remove_dir_all(fixture.original).unwrap();
    fs::remove_dir_all(fixture.held).unwrap();
}

fn tree_digest(root: &Path) -> [u8; 32] {
    let mut entries = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();
    let mut hash = Sha256::new();
    for path in entries {
        hash.update(path.file_name().unwrap().as_encoded_bytes());
        if path.is_dir() {
            hash.update(tree_digest(&path));
        } else {
            hash.update(fs::read(path).unwrap());
        }
    }
    hash.finalize().into()
}
