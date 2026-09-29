use std::fs;

use super::owned_compile_retention::{CleanupDirective, CleanupOutcome, CleanupStage};
use super::owned_compile_scratch::OwnedCompileScratch;

#[test]
fn capture_to_former_rename_pause_cannot_move_any_entry() {
    let mut owned = OwnedCompileScratch::claim("routine-retained-capture");
    let sentinel = owned.path().join("sentinel");
    fs::write(&sentinel, b"captured bytes\n").unwrap();
    let outcome = owned.recover_with(|stage| {
        assert!(matches!(
            stage,
            CleanupStage::RootQuarantineRenameRetired
                | CleanupStage::ForeignRecoveryRenameRetired
                | CleanupStage::RootDirectoryUnlinkRetired
                | CleanupStage::DescendantDirectoryUnlinkRetired
                | CleanupStage::DescendantEntryUnlinkRetired
        ));
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::RetainedNoDestructiveAuthority);
    assert_eq!(fs::read(&sentinel).unwrap(), b"captured bytes\n");
    owned.teardown_after_assertions();
}

#[test]
fn root_replacement_at_first_retired_transition_is_never_consumed() {
    let mut owned = OwnedCompileScratch::claim("routine-retained-capture-swap");
    let original = owned.path().to_path_buf();
    let held = original.with_extension("capture-held");
    fs::write(original.join("captured"), b"captured bytes\n").unwrap();
    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::RootQuarantineRenameRetired {
            fs::rename(&original, &held).unwrap();
            fs::create_dir(&original).unwrap();
            fs::write(original.join("attacker"), b"attacker bytes\n").unwrap();
        }
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::RefusedZeroWrite);
    assert_eq!(
        fs::read(held.join("captured")).unwrap(),
        b"captured bytes\n"
    );
    assert_eq!(
        fs::read(original.join("attacker")).unwrap(),
        b"attacker bytes\n"
    );
    fs::remove_dir_all(original).unwrap();
    fs::remove_dir_all(held).unwrap();
}
