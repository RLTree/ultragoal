use std::fs;

use super::owned_compile_retention::{CleanupDirective, CleanupOutcome, CleanupStage};
use super::owned_compile_scratch::OwnedCompileScratch;

#[test]
fn initial_quarantine_boundary_has_no_rename_authority() {
    let mut owned = OwnedCompileScratch::claim("routine-retained-quarantine");
    let original = owned.path().to_path_buf();
    let held = original.with_extension("quarantine-held");
    fs::write(original.join("owned"), b"owned bytes\n").unwrap();
    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::RootQuarantineRenameRetired {
            fs::rename(&original, &held).unwrap();
            fs::create_dir(&original).unwrap();
            fs::write(original.join("replacement"), b"replacement bytes\n").unwrap();
        }
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::RefusedZeroWrite);
    assert_eq!(fs::read(held.join("owned")).unwrap(), b"owned bytes\n");
    assert_eq!(
        fs::read(original.join("replacement")).unwrap(),
        b"replacement bytes\n"
    );
    fs::remove_dir_all(original).unwrap();
    fs::remove_dir_all(held).unwrap();
}

#[test]
fn replay_retains_exact_tree_without_mutation() {
    let mut owned = OwnedCompileScratch::claim("routine-retained-replay");
    let sentinel = owned.path().join("sentinel");
    fs::write(&sentinel, b"replay sentinel\n").unwrap();
    for _ in 0..4 {
        assert_eq!(
            owned.recover_interrupted(),
            CleanupOutcome::RetainedNoDestructiveAuthority
        );
        assert_eq!(fs::read(&sentinel).unwrap(), b"replay sentinel\n");
    }
    owned.teardown_after_assertions();
}

#[test]
fn drop_performs_no_hidden_cleanup() {
    let owned = OwnedCompileScratch::claim("routine-retained-drop");
    let path = owned.path().to_path_buf();
    fs::write(path.join("sentinel"), b"drop sentinel\n").unwrap();
    drop(owned);
    assert_eq!(fs::read(path.join("sentinel")).unwrap(), b"drop sentinel\n");
    fs::remove_dir_all(path).unwrap();
}
