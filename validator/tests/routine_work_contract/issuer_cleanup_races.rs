use std::fs;
use std::path::Path;
use std::sync::{Arc, Barrier};

use sha2::{Digest, Sha256};

use super::owned_compile_quarantine::{CleanupDirective, CleanupOutcome, CleanupStage};
use super::owned_compile_scratch::OwnedCompileScratch;

#[test]
fn authenticate_to_quarantine_swap_restores_foreign_without_content_change() {
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
    assert_eq!(outcome, CleanupOutcome::AmbiguousPartialEffect);
    assert_eq!(
        owned.recover_interrupted(),
        CleanupOutcome::ReconciledForeign
    );
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
fn failed_restore_retains_exact_foreign_custody_until_safe_reconciliation() {
    let mut owned = OwnedCompileScratch::claim("routine-issuer-restore-owner");
    let mut attacker = OwnedCompileScratch::claim("routine-issuer-restore-attacker");
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
    let foreign_quarantined = Arc::new(Barrier::new(2));
    let swap_done = Arc::new(Barrier::new(2));
    let refill_done = Arc::new(Barrier::new(2));
    let worker_original = original.clone();
    let worker_attacker = attacker_original.clone();
    let worker_held = held.clone();
    let worker_authenticated = Arc::clone(&authenticated);
    let worker_foreign = Arc::clone(&foreign_quarantined);
    let worker_swap_done = Arc::clone(&swap_done);
    let worker_refill_done = Arc::clone(&refill_done);
    let adversary = std::thread::spawn(move || {
        worker_authenticated.wait();
        fs::rename(&worker_original, worker_held).unwrap();
        fs::rename(worker_attacker, &worker_original).unwrap();
        worker_swap_done.wait();
        worker_foreign.wait();
        fs::write(&worker_original, b"refilled blocker\n").unwrap();
        worker_refill_done.wait();
    });

    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::Authenticated {
            authenticated.wait();
            swap_done.wait();
        } else if stage == CleanupStage::ForeignQuarantined {
            foreign_quarantined.wait();
            refill_done.wait();
        }
        CleanupDirective::Continue
    });
    adversary.join().unwrap();
    assert_eq!(outcome, CleanupOutcome::AmbiguousPartialEffect);
    let quarantine = owned.recovery_path().unwrap();
    assert_eq!(tree_digest(&held), genuine_before);
    assert_eq!(tree_digest(&quarantine), attacker_before);
    assert_eq!(fs::read(&original).unwrap(), b"refilled blocker\n");

    assert_eq!(
        owned.recover_interrupted(),
        CleanupOutcome::AmbiguousPartialEffect
    );
    assert_eq!(owned.recovery_path().as_deref(), Some(quarantine.as_path()));
    let transplant = quarantine.with_extension("transplanted");
    fs::rename(&quarantine, &transplant).unwrap();
    fs::create_dir(&quarantine).unwrap();
    fs::write(quarantine.join("replacement"), b"replacement bytes\n").unwrap();
    assert_eq!(
        owned.recover_interrupted(),
        CleanupOutcome::AmbiguousPartialEffect
    );
    assert_eq!(tree_digest(&transplant), attacker_before);
    assert_eq!(owned.recovery_path().as_deref(), Some(transplant.as_path()));
    assert_eq!(
        fs::read(quarantine.join("replacement")).unwrap(),
        b"replacement bytes\n"
    );
    assert_eq!(tree_digest(&held), genuine_before);
    assert_eq!(fs::read(&original).unwrap(), b"refilled blocker\n");

    fs::remove_dir_all(&quarantine).unwrap();
    fs::remove_file(&original).unwrap();
    assert_eq!(
        owned.recover_interrupted(),
        CleanupOutcome::ReconciledForeign
    );
    assert!(!transplant.exists());
    assert_eq!(tree_digest(&original), attacker_before);
    assert_eq!(tree_digest(&held), genuine_before);
    assert_eq!(
        owned.recover_interrupted(),
        CleanupOutcome::RefusedZeroWrite
    );
    assert_eq!(
        attacker.recover_interrupted(),
        CleanupOutcome::RefusedZeroWrite
    );
    assert_eq!(tree_digest(&original), attacker_before);
    assert_eq!(tree_digest(&held), genuine_before);
    fs::remove_dir_all(original).unwrap();
    fs::remove_dir_all(held).unwrap();
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
