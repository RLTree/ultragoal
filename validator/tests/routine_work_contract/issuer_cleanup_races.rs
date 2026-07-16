use std::fs;

use super::owned_compile_retention::{CleanupDirective, CleanupOutcome, CleanupStage};
use super::owned_compile_scratch::OwnedCompileScratch;

#[test]
fn root_unlink_boundary_refuses_after_name_substitution_without_deletion() {
    let mut owned = OwnedCompileScratch::claim("routine-retained-root");
    let original = owned.path().to_path_buf();
    let held = original.with_extension("held");
    fs::write(original.join("owned"), b"owned bytes\n").unwrap();
    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::RootDirectoryUnlinkRetired {
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
fn nested_directory_boundary_preserves_checked_and_replacement_trees() {
    let mut owned = OwnedCompileScratch::claim("routine-retained-nested");
    let nested = owned.path().join("nested");
    let held = owned.path().join("nested-held");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("owned"), b"nested owned\n").unwrap();
    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::DescendantDirectoryUnlinkRetired {
            fs::rename(&nested, &held).unwrap();
            fs::create_dir(&nested).unwrap();
            fs::write(nested.join("replacement"), b"nested replacement\n").unwrap();
        }
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::RetainedNoDestructiveAuthority);
    assert_eq!(fs::read(held.join("owned")).unwrap(), b"nested owned\n");
    assert_eq!(
        fs::read(nested.join("replacement")).unwrap(),
        b"nested replacement\n"
    );
    owned.teardown_after_assertions();
}

#[test]
fn regular_file_boundary_preserves_checked_and_replacement_files() {
    let mut owned = OwnedCompileScratch::claim("routine-retained-file");
    let file = owned.path().join("artifact");
    let held = owned.path().join("artifact-held");
    fs::write(&file, b"owned file\n").unwrap();
    let outcome = owned.recover_with(|stage| {
        if stage == CleanupStage::DescendantEntryUnlinkRetired {
            fs::rename(&file, &held).unwrap();
            fs::write(&file, b"replacement file\n").unwrap();
        }
        CleanupDirective::Continue
    });
    assert_eq!(outcome, CleanupOutcome::RetainedNoDestructiveAuthority);
    assert_eq!(fs::read(&held).unwrap(), b"owned file\n");
    assert_eq!(fs::read(&file).unwrap(), b"replacement file\n");
    owned.teardown_after_assertions();
}

#[test]
fn interrupted_retired_transition_is_repeatably_zero_effect() {
    let mut owned = OwnedCompileScratch::claim("routine-retained-interruption");
    fs::write(owned.path().join("sentinel"), b"sentinel\n").unwrap();
    let outcome = owned.recover_with(|stage| {
        (stage != CleanupStage::DescendantEntryUnlinkRetired)
            .then_some(CleanupDirective::Continue)
            .unwrap_or(CleanupDirective::Interrupt)
    });
    assert_eq!(outcome, CleanupOutcome::RefusedZeroWrite);
    assert_eq!(
        fs::read(owned.path().join("sentinel")).unwrap(),
        b"sentinel\n"
    );
    assert_eq!(
        owned.recover_interrupted(),
        CleanupOutcome::RetainedNoDestructiveAuthority
    );
    owned.teardown_after_assertions();
}
