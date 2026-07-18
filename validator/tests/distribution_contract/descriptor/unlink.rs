#![cfg(unix)]

use crate::distribution::{
    DistributionErrorId as ErrorId, EffectPoint, ExpectedTree, ScopedFile, ScopedTree,
    assert_test_effect_hook_consumed, materialize_package, set_test_effect_hook_matching,
};
use crate::distribution_fixture::digest;
use crate::package_journey_fixture::{JourneyFixture, write_scoped};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

#[test]
fn pre_quarantine_substitution_is_restored_and_never_deleted() {
    let fixture = JourneyFixture::new("descriptor-pre-quarantine");
    let file = write_scoped(fixture.confined(), "state/value.bin", b"expected");
    let paths = Rc::new(RefCell::new(None));
    let recorded = paths.clone();
    let root = fixture.root.clone();
    set_test_effect_hook_matching(EffectPoint::Quarantine, ".hul-stage-", move |detail| {
        let original = rooted(&root, detail);
        let saved = original.with_extension("expected-saved");
        fs::rename(&original, &saved).unwrap();
        fs::write(&original, b"attacker-before-linearization").unwrap();
        recorded.replace(Some((original, saved)));
    });
    assert!(file.apply(Some(&digest(b"expected")), None).is_err());
    assert_test_effect_hook_consumed();
    let (attacker, expected) = paths.borrow().clone().unwrap();
    assert_eq!(
        fs::read(&attacker).unwrap(),
        b"attacker-before-linearization"
    );
    assert_eq!(fs::read(&expected).unwrap(), b"expected");
    fs::remove_file(attacker).unwrap();
    fs::remove_file(expected).unwrap();
}

#[test]
fn post_quarantine_substitution_is_restored_and_never_deleted() {
    let fixture = JourneyFixture::new("descriptor-post-quarantine");
    let file = write_scoped(fixture.confined(), "state/value.bin", b"expected");
    let paths = Rc::new(RefCell::new(None));
    let recorded = paths.clone();
    let root = fixture.root.clone();
    set_test_effect_hook_matching(EffectPoint::Unlink, ".hul-quarantine-", move |detail| {
        let quarantine = rooted(&root, detail);
        let saved = quarantine.with_extension("expected-saved");
        fs::rename(&quarantine, &saved).unwrap();
        fs::write(&quarantine, b"attacker-after-linearization").unwrap();
        recorded.replace(Some((quarantine, saved)));
    });
    assert!(file.apply(Some(&digest(b"expected")), None).is_err());
    assert_test_effect_hook_consumed();
    let (attacker, expected) = paths.borrow().clone().unwrap();
    assert_eq!(
        fs::read(&attacker).unwrap(),
        b"attacker-after-linearization"
    );
    assert_eq!(fs::read(&expected).unwrap(), b"expected");
    fs::remove_file(attacker).unwrap();
    fs::remove_file(expected).unwrap();
}

#[test]
fn attacker_target_created_after_quarantine_survives_failed_final_revalidation() {
    let fixture = JourneyFixture::new("descriptor-post-quarantine-original");
    let file = write_scoped(fixture.confined(), "state/value.bin", b"expected");
    let target = fixture.root.join("state/value.bin");
    let target_for_hook = target.clone();
    let root = fixture.root.clone();
    set_test_effect_hook_matching(EffectPoint::Unlink, ".hul-quarantine-", move |_| {
        assert!(root.exists());
        fs::write(&target_for_hook, b"attacker-after-linearization").unwrap();
    });
    assert!(file.apply(Some(&digest(b"expected")), None).is_err());
    assert_test_effect_hook_consumed();
    assert_eq!(fs::read(&target).unwrap(), b"attacker-after-linearization");
}

#[test]
fn disposal_collision_restores_the_expected_name_and_preserves_the_collision() {
    let fixture = JourneyFixture::new("descriptor-disposal-collision");
    let file = write_scoped(fixture.confined(), "state/value.bin", b"expected");
    let target = fixture.root.join("state/value.bin");
    let collision = Rc::new(RefCell::new(None));
    let recorded = collision.clone();
    let root = fixture.root.clone();
    set_test_effect_hook_matching(EffectPoint::Unlink, ".hul-quarantine-", move |detail| {
        let quarantine = rooted(&root, detail);
        let name = quarantine.file_name().unwrap().to_string_lossy();
        let disposal =
            quarantine.with_file_name(name.replacen(".hul-quarantine-", ".hul-disposal-", 1));
        fs::write(&disposal, b"foreign-disposal").unwrap();
        recorded.replace(Some(disposal));
    });
    assert_eq!(
        file.apply(Some(&digest(b"expected")), None)
            .unwrap_err()
            .id(),
        ErrorId::ObjectChanged,
    );
    assert_test_effect_hook_consumed();
    assert_eq!(fs::read(target).unwrap(), b"expected");
    assert_eq!(
        fs::read(collision.borrow().as_ref().unwrap()).unwrap(),
        b"foreign-disposal"
    );
}

#[test]
fn lock_drop_quarantine_preserves_same_parent_substitution() {
    let fixture = JourneyFixture::new("descriptor-lock-drop");
    let file = ScopedFile::new(fixture.confined(), "state/value.bin").unwrap();
    let paths = Rc::new(RefCell::new(None));
    let recorded = paths.clone();
    let root = fixture.root.clone();
    set_test_effect_hook_matching(EffectPoint::Quarantine, ".hul-lock-", move |detail| {
        let lock = rooted(&root, detail);
        let saved = lock.with_extension("expected-saved");
        fs::rename(&lock, &saved).unwrap();
        fs::write(&lock, b"attacker-lock").unwrap();
        recorded.replace(Some((lock, saved)));
    });
    assert!(
        !file
            .apply(Some(&digest(b"missing")), Some(b"candidate"))
            .unwrap()
    );
    assert_test_effect_hook_consumed();
    let (attacker, expected) = paths.borrow().clone().unwrap();
    assert_eq!(fs::read(&attacker).unwrap(), b"attacker-lock");
    assert_eq!(fs::read(&expected).unwrap(), b"");
    fs::remove_file(attacker).unwrap();
    fs::remove_file(expected).unwrap();
}

#[test]
fn recursive_tree_cleanup_preserves_same_parent_file_substitution() {
    let fixture = JourneyFixture::new("descriptor-tree-cleanup");
    let plan = fixture.plan();
    let mut tree = ScopedTree::new(fixture.confined(), "installed/tree").unwrap();
    materialize_package(&plan, &ExpectedTree::Absent, &mut tree).unwrap();
    let paths = Rc::new(RefCell::new(None));
    let recorded = paths.clone();
    let root = fixture.root.clone();
    set_test_effect_hook_matching(EffectPoint::Quarantine, "plugin.json", move |detail| {
        let object = rooted(&root, detail);
        let saved = object.with_extension("expected-saved");
        fs::rename(&object, &saved).unwrap();
        fs::write(&object, b"attacker-tree-object").unwrap();
        recorded.replace(Some((object, saved)));
    });
    assert!(
        materialize_package(
            &plan,
            &ExpectedTree::ExactDigest(plan.source_tree_sha256().into()),
            &mut tree,
        )
        .is_err()
    );
    assert_test_effect_hook_consumed();
    let (attacker, expected) = paths.borrow().clone().unwrap();
    assert_eq!(fs::read(&attacker).unwrap(), b"attacker-tree-object");
    assert_ne!(fs::read(&expected).unwrap(), b"attacker-tree-object");
}

fn rooted(root: &std::path::Path, detail: &str) -> PathBuf {
    let mut components = detail.split('/');
    let _root_name = components.next().unwrap();
    components.fold(root.to_path_buf(), |path, component| path.join(component))
}
