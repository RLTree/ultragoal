use std::fs;
use std::path::Path;
use std::sync::{Arc, Barrier};

use sha2::{Digest, Sha256};

use super::owned_compile_quarantine::{CleanupDirective, CleanupOutcome, CleanupStage};
use super::owned_compile_scratch::OwnedCompileScratch;

#[test]
fn authenticate_to_quarantine_swap_is_a_byte_stable_refusal() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-race-owner");
    let mut attacker = OwnedCompileScratch::claim("routine-issuer-race-attacker");
    let original = owned.path().to_path_buf();
    let attacker_original = attacker.path().to_path_buf();
    let held = original.with_file_name(format!(
        "{}-held",
        original.file_name().unwrap().to_string_lossy()
    ));
    fs::write(original.join("genuine"), b"genuine bytes\n").unwrap();
    fs::write(attacker_original.join("attacker"), b"attacker bytes\n").unwrap();
    let genuine_before = tree_digest(&original);
    let attacker_before = tree_digest(&attacker_original);
    let authenticated = Arc::new(Barrier::new(2));
    let swapped = Arc::new(Barrier::new(2));
    let worker_authenticated = Arc::clone(&authenticated);
    let worker_swapped = Arc::clone(&swapped);
    let worker_original = original.clone();
    let worker_attacker = attacker_original.clone();
    let worker_held = held.clone();
    let adversary = std::thread::spawn(move || {
        worker_authenticated.wait();
        fs::rename(&worker_original, &worker_held).unwrap();
        fs::rename(worker_attacker, worker_original).unwrap();
        worker_swapped.wait();
    });

    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::Authenticated {
            authenticated.wait();
            swapped.wait();
        }
        CleanupDirective::Continue
    });
    adversary.join().unwrap();
    assert_eq!(outcome, CleanupOutcome::RefusedZeroWrite);
    assert_eq!(tree_digest(&held), genuine_before);
    assert_eq!(tree_digest(&original), attacker_before);
    assert_eq!(
        attacker.recover_interrupted(),
        CleanupOutcome::RefusedZeroWrite
    );
    assert_eq!(tree_digest(&held), genuine_before);
    assert_eq!(tree_digest(&original), attacker_before);
    fs::remove_dir_all(held).unwrap();
    fs::remove_dir_all(original).unwrap();
}

#[test]
fn quarantine_interruption_is_ambiguous_then_replay_deletes_only_genuine() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-quarantine-interruption");
    let mut unrelated = OwnedCompileScratch::claim("routine-issuer-quarantine-unrelated");
    let original = owned.path().to_path_buf();
    let unrelated_path = unrelated.path().to_path_buf();
    fs::write(original.join("genuine"), b"genuine bytes\n").unwrap();
    fs::write(unrelated_path.join("unrelated"), b"unrelated bytes\n").unwrap();
    let genuine_before = tree_digest(&original);
    let unrelated_before = tree_digest(&unrelated_path);

    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::Quarantined {
            CleanupDirective::Interrupt
        } else {
            CleanupDirective::Continue
        }
    });
    assert_eq!(outcome, CleanupOutcome::AmbiguousPartialEffect);
    let quarantine = owned.recovery_path().unwrap();
    assert!(!original.exists());
    assert_eq!(tree_digest(&quarantine), genuine_before);
    assert_eq!(tree_digest(&unrelated_path), unrelated_before);

    assert_eq!(owned.recover_interrupted(), CleanupOutcome::Deleted);
    assert!(!quarantine.exists());
    assert_eq!(tree_digest(&unrelated_path), unrelated_before);
    assert_eq!(unrelated.recover_interrupted(), CleanupOutcome::Deleted);
}

#[test]
fn cleared_interruption_is_ambiguous_then_repeat_unlinks_empty_quarantine() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-cleared-interruption");
    let original = owned.path().to_path_buf();
    fs::create_dir_all(original.join("nested")).unwrap();
    fs::write(original.join("nested/artifact"), b"bounded bytes\n").unwrap();

    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::Cleared {
            CleanupDirective::Interrupt
        } else {
            CleanupDirective::Continue
        }
    });
    assert_eq!(outcome, CleanupOutcome::AmbiguousPartialEffect);
    let quarantine = owned.recovery_path().unwrap();
    assert!(quarantine.is_dir());
    assert_eq!(fs::read_dir(&quarantine).unwrap().count(), 0);

    assert_eq!(owned.recover_interrupted(), CleanupOutcome::Deleted);
    assert!(!quarantine.exists());
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
