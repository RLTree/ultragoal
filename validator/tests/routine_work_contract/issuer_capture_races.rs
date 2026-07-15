use std::fs;
use std::os::unix::fs::{MetadataExt, symlink};
use std::path::Path;

use sha2::{Digest, Sha256};

use super::owned_compile_quarantine::{CleanupDirective, CleanupOutcome, CleanupStage};
use super::owned_compile_scratch::OwnedCompileScratch;

#[test]
fn capture_to_quarantine_swap_pins_the_entry_actually_moved() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-capture-owner");
    let mut replacement = OwnedCompileScratch::claim("routine-issuer-capture-replacement");
    let original = owned.path().to_path_buf();
    let replacement_path = replacement.path().to_path_buf();
    let held = original.with_file_name(format!(
        "{}-captured-held",
        original.file_name().unwrap().to_string_lossy()
    ));
    fs::write(original.join("captured"), b"captured bytes\n").unwrap();
    fs::write(replacement_path.join("moved"), b"moved bytes\n").unwrap();
    let captured_digest = tree_digest(&original);
    let moved_digest = tree_digest(&replacement_path);

    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::ForeignCaptured {
            fs::rename(&original, &held).unwrap();
            fs::rename(&replacement_path, &original).unwrap();
        }
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::AmbiguousPartialEffect);
    let moved_quarantine = owned.recovery_path().unwrap();
    assert_eq!(tree_digest(&moved_quarantine), moved_digest);
    assert_eq!(tree_digest(&held), captured_digest);
    assert!(!original.exists());

    assert_eq!(
        owned.recover_interrupted(),
        CleanupOutcome::ReconciledForeign
    );
    assert_eq!(tree_digest(&original), moved_digest);
    assert_eq!(tree_digest(&held), captured_digest);
    assert!(!moved_quarantine.exists());
    assert_eq!(
        replacement.recover_interrupted(),
        CleanupOutcome::RefusedZeroWrite
    );
    fs::remove_dir_all(original).unwrap();
    fs::remove_dir_all(held).unwrap();
}

#[test]
fn unpinnable_moved_entry_stays_unrecoverable_without_touching_substitutes() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-unpinned-owner");
    let replacement = OwnedCompileScratch::claim("routine-issuer-unpinned-replacement");
    let original = owned.path().to_path_buf();
    let replacement_path = replacement.path().to_path_buf();
    let root = original.parent().unwrap().to_path_buf();
    let held = original.with_extension("captured-held");
    fs::write(original.join("captured"), b"captured bytes\n").unwrap();
    fs::write(replacement_path.join("moved"), b"moved bytes\n").unwrap();
    let replacement_identity = (replacement.device, replacement.inode);
    let mut transplant = None;

    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::ForeignCaptured {
            fs::rename(&original, &held).unwrap();
            fs::rename(&replacement_path, &original).unwrap();
        } else if stage == CleanupStage::ForeignMoveMismatch {
            let quarantine = find_identity(&root, replacement_identity);
            let displaced = quarantine.with_extension("unpinned");
            fs::rename(&quarantine, &displaced).unwrap();
            symlink(&held, &quarantine).unwrap();
            transplant = Some((quarantine, displaced));
        }
        CleanupDirective::Continue
    });
    let (quarantine, displaced) = transplant.unwrap();
    assert_eq!(outcome, CleanupOutcome::AmbiguousPartialEffect);
    assert_eq!(
        owned.recover_interrupted(),
        CleanupOutcome::AmbiguousPartialEffect
    );
    assert!(
        fs::symlink_metadata(&quarantine)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(displaced.join("moved")).unwrap(), b"moved bytes\n");
    assert_eq!(
        fs::read(held.join("captured")).unwrap(),
        b"captured bytes\n"
    );
    fs::remove_file(quarantine).unwrap();
    fs::remove_dir_all(displaced).unwrap();
    fs::remove_dir_all(held).unwrap();
}

fn find_identity(root: &Path, identity: (u64, u64)) -> std::path::PathBuf {
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
