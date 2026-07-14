#![cfg(unix)]

use crate::distribution::{
    DistributionErrorId as ErrorId, EffectPoint, ScopedFile, ScopedTree,
    assert_test_effect_hook_consumed, set_test_effect_hook_matching,
};
use crate::distribution_fixture::digest;
use crate::package_journey_fixture::{JourneyFixture, write_scoped};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

#[test]
fn exact_pre_rename_content_substitution_is_a_cas_conflict_and_is_not_overwritten() {
    let fixture = JourneyFixture::new("descriptor-file-cas");
    let file = write_scoped(fixture.confined(), "state/value.bin", b"expected");
    let target = fixture.root.join("state/value.bin");
    let replacement = fixture.root.join("state/concurrent.bin");
    set_test_effect_hook_matching(EffectPoint::Rename, "", move |_| {
        fs::write(&replacement, b"concurrent").unwrap();
        fs::rename(&replacement, &target).unwrap();
    });
    assert!(
        !file
            .apply(Some(&digest(b"expected")), Some(b"candidate"))
            .unwrap()
    );
    assert_test_effect_hook_consumed();
    assert_eq!(file.inspect(64).unwrap().unwrap(), b"concurrent");
    assert!(fs::read_dir(&fixture.root).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".hul-stage-")
    }));
}

#[test]
fn replacement_stage_substitution_is_rolled_back_without_replacing_the_target() {
    let fixture = JourneyFixture::new("descriptor-file-stage-cas");
    let file = write_scoped(fixture.confined(), "state/value.bin", b"expected");
    let target = fixture.root.join("state/value.bin");
    let paths = Rc::new(RefCell::new(None));
    let recorded = paths.clone();
    let root = fixture.root.clone();
    set_test_effect_hook_matching(EffectPoint::Rename, ".hul-stage-", move |detail| {
        let relative = detail.split("<->").next().unwrap();
        let stage = rooted(&root, relative);
        let saved = stage.with_file_name(format!(
            "{}-expected-saved",
            stage.file_name().unwrap().to_string_lossy()
        ));
        fs::rename(&stage, &saved).unwrap();
        fs::write(&stage, b"attacker-stage").unwrap();
        recorded.replace(Some((stage, saved)));
    });
    assert!(
        file.apply(Some(&digest(b"expected")), Some(b"candidate"))
            .is_err()
    );
    assert_test_effect_hook_consumed();
    let (attacker, candidate) = paths.borrow().clone().unwrap();
    assert_eq!(fs::read(target).unwrap(), b"expected");
    assert_eq!(fs::read(attacker).unwrap(), b"attacker-stage");
    assert_eq!(fs::read(candidate).unwrap(), b"candidate");
}

#[test]
fn absent_target_stage_substitution_restores_absence_and_preserves_both_files() {
    let fixture = JourneyFixture::new("descriptor-file-absent-cas");
    let file = ScopedFile::new(fixture.confined(), "state/value.bin").unwrap();
    let target = fixture.root.join("state/value.bin");
    let paths = Rc::new(RefCell::new(None));
    let recorded = paths.clone();
    let root = fixture.root.clone();
    set_test_effect_hook_matching(EffectPoint::Rename, ".hul-stage-", move |detail| {
        let relative = detail.split("->").next().unwrap();
        let stage = rooted(&root, relative);
        let saved = stage.with_file_name(format!(
            "{}-expected-saved",
            stage.file_name().unwrap().to_string_lossy()
        ));
        fs::rename(&stage, &saved).unwrap();
        fs::write(&stage, b"attacker-stage").unwrap();
        recorded.replace(Some((stage, saved)));
    });
    assert!(file.apply(None, Some(b"candidate")).is_err());
    assert_test_effect_hook_consumed();
    let (attacker, candidate) = paths.borrow().clone().unwrap();
    assert!(!target.exists());
    assert_eq!(fs::read(attacker).unwrap(), b"attacker-stage");
    assert_eq!(fs::read(candidate).unwrap(), b"candidate");
}

#[test]
fn descriptor_enumeration_rejects_more_than_the_internal_name_budget() {
    let fixture = JourneyFixture::new("descriptor-name-budget");
    let directory = fixture.root.join("overflow");
    fs::create_dir(&directory).unwrap();
    for index in 0..=4096 {
        fs::write(directory.join(format!("entry-{index:04}")), []).unwrap();
    }
    let tree = ScopedTree::new(fixture.confined(), "overflow").unwrap();
    assert_eq!(
        tree.inspect(usize::MAX, usize::MAX).unwrap_err().id(),
        ErrorId::ObjectTooLarge,
    );
}

fn rooted(root: &std::path::Path, detail: &str) -> PathBuf {
    let mut components = detail.split('/');
    let _root_name = components.next().unwrap();
    components.fold(root.to_path_buf(), |path, component| path.join(component))
}
