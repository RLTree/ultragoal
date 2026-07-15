use std::cell::RefCell;
use std::ffi::CString;
use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt, symlink};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::owned_compile_quarantine::{CleanupDirective, CleanupOutcome, CleanupStage};
use super::owned_compile_scratch::OwnedCompileScratch;

#[test]
fn rollback_check_to_rename_symlink_swap_is_ambiguous_and_recoverable() {
    let mut fixture = rollback_fixture("routine-issuer-rollback-symlink", Scenario::Symlink);
    assert_eq!(fixture.outcome, CleanupOutcome::AmbiguousPartialEffect);
    assert!(
        fs::symlink_metadata(&fixture.quarantine)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(tree_digest(&fixture.transplant), fixture.foreign_digest);
    assert_eq!(tree_digest(&fixture.held), fixture.genuine_digest);
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::AmbiguousPartialEffect
    );
    fs::remove_file(&fixture.quarantine).unwrap();
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::ReconciledForeign
    );
    finish_fixture(fixture);
}

#[test]
fn rollback_quarantine_disappearance_retains_exact_custody() {
    let mut fixture =
        rollback_fixture("routine-issuer-rollback-disappearance", Scenario::Disappear);
    assert_eq!(fixture.outcome, CleanupOutcome::AmbiguousPartialEffect);
    assert!(!fixture.original.exists());
    assert!(!fixture.quarantine.exists());
    assert_eq!(tree_digest(&fixture.transplant), fixture.foreign_digest);
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::ReconciledForeign
    );
    finish_fixture(fixture);
}

#[test]
fn rollback_postmove_fifo_replacement_never_reports_restored() {
    let mut fixture = rollback_fixture("routine-issuer-rollback-fifo", Scenario::Fifo);
    assert_eq!(fixture.outcome, CleanupOutcome::AmbiguousPartialEffect);
    assert!(
        fs::symlink_metadata(&fixture.original)
            .unwrap()
            .file_type()
            .is_fifo()
    );
    assert_eq!(tree_digest(&fixture.transplant), fixture.foreign_digest);
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::AmbiguousPartialEffect
    );
    assert!(fixture.owned.ambiguous_destination().unwrap().is_some());
    fs::remove_file(&fixture.original).unwrap();
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::ReconciledForeign
    );
    finish_fixture(fixture);
}

#[test]
fn rollback_postmove_quarantine_refill_blocks_restored_outcome() {
    let mut fixture = rollback_fixture("routine-issuer-rollback-refill", Scenario::Refill);
    assert_eq!(fixture.outcome, CleanupOutcome::AmbiguousPartialEffect);
    assert!(!fixture.original.exists());
    assert_eq!(
        fs::read(fixture.quarantine.join("unrelated")).unwrap(),
        b"unrelated bytes\n"
    );
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::AmbiguousPartialEffect
    );
    fs::remove_dir_all(&fixture.quarantine).unwrap();
    assert_eq!(
        fixture.owned.recover_interrupted(),
        CleanupOutcome::ReconciledForeign
    );
    finish_fixture(fixture);
}

#[derive(Clone, Copy)]
enum Scenario {
    Symlink,
    Disappear,
    Fifo,
    Refill,
}

struct RollbackFixture {
    owned: OwnedCompileScratch,
    foreign: OwnedCompileScratch,
    original: PathBuf,
    held: PathBuf,
    quarantine: PathBuf,
    transplant: PathBuf,
    genuine_digest: [u8; 32],
    foreign_digest: [u8; 32],
    outcome: CleanupOutcome,
}

fn rollback_fixture(label: &str, scenario: Scenario) -> RollbackFixture {
    let mut owned = OwnedCompileScratch::claim(&format!("{label}-owner"));
    let foreign = OwnedCompileScratch::claim(&format!("{label}-foreign"));
    let original = owned.path().to_path_buf();
    let foreign_original = foreign.path().to_path_buf();
    let root = original.parent().unwrap().to_path_buf();
    let held = original.with_file_name(format!(
        "{}-held",
        original.file_name().unwrap().to_string_lossy()
    ));
    fs::write(original.join("genuine"), b"genuine bytes\n").unwrap();
    fs::write(foreign_original.join("foreign"), b"foreign bytes\n").unwrap();
    let genuine_digest = tree_digest(&original);
    let foreign_digest = tree_digest(&foreign_original);
    let foreign_identity = (foreign.device, foreign.inode);
    let quarantine = RefCell::new(None::<PathBuf>);
    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::Authenticated {
            fs::rename(&original, &held).unwrap();
            fs::rename(&foreign_original, &original).unwrap();
        } else if stage == CleanupStage::ForeignQuarantined {
            let path = find_identity(&root, foreign_identity);
            *quarantine.borrow_mut() = Some(path.clone());
            let transplant = path.with_extension("transplanted");
            fs::rename(&path, &transplant).unwrap();
            match scenario {
                Scenario::Symlink => symlink(&held, &path).unwrap(),
                Scenario::Disappear | Scenario::Fifo | Scenario::Refill => {}
            }
            if matches!(scenario, Scenario::Fifo) {
                let name = CString::new(original.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            } else if matches!(scenario, Scenario::Refill) {
                fs::create_dir(&path).unwrap();
                fs::write(path.join("unrelated"), b"unrelated bytes\n").unwrap();
            }
        }
        CleanupDirective::Continue
    });
    let quarantine = quarantine.into_inner().unwrap();
    let transplant = quarantine.with_extension("transplanted");
    RollbackFixture {
        owned,
        foreign,
        original,
        held,
        quarantine,
        transplant,
        genuine_digest,
        foreign_digest,
        outcome,
    }
}

fn finish_fixture(mut fixture: RollbackFixture) {
    assert!(!fixture.quarantine.exists());
    assert!(!fixture.transplant.exists());
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

fn find_identity(root: &Path, identity: (u64, u64)) -> PathBuf {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            fs::symlink_metadata(path)
                .map(|metadata| (metadata.dev(), metadata.ino()) == identity)
                .unwrap_or(false)
        })
        .unwrap()
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
