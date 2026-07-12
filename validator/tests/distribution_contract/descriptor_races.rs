#![cfg(unix)]

use crate::distribution::{
    DistributionErrorId as ErrorId, EffectPoint, ExpectedTree, ScopedFile, ScopedTree,
    assert_test_effect_hook_consumed, materialize_package, set_test_effect_hook,
    set_test_effect_hook_matching,
};
use crate::journey_support::{JourneyFixture, renamed, write_scoped};
use crate::support::{digest, tree};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_OUTSIDE: AtomicU64 = AtomicU64::new(0);

struct Outside(PathBuf);

impl Outside {
    fn new(label: &str) -> Self {
        let path = PathBuf::from("/tmp").join(format!(
            "hul-distribution-descriptor-{label}-{}-{}-{}",
            std::process::id(),
            NEXT_OUTSIDE.fetch_add(1, Ordering::Relaxed),
            unique_time(),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn snapshot(&self) -> Vec<(String, Vec<u8>)> {
        tree(&self.0)
    }
}

fn unique_time() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

impl Drop for Outside {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn ancestor_symlink_swap_before_file_rename_never_mutates_outside_leaf() {
    use std::os::unix::fs::symlink;
    for outside_present in [false, true] {
        let fixture = JourneyFixture::new("descriptor-ancestor-rename");
        let outside = Outside::new("ancestor-rename");
        fs::create_dir(fixture.root.join("inside")).unwrap();
        if outside_present {
            fs::write(outside.0.join("value.bin"), b"outside-present").unwrap();
        }
        let file = write_scoped(fixture.confined(), "inside/value.bin", b"old");
        let outside_before = outside.snapshot();
        let inside = fixture.root.join("inside");
        let original = fixture.root.join("inside-original");
        let outside_path = outside.0.clone();
        let inside_for_hook = inside.clone();
        let original_for_hook = original.clone();
        set_test_effect_hook(EffectPoint::Rename, move |_| {
            fs::rename(&inside_for_hook, &original_for_hook).unwrap();
            symlink(&outside_path, &inside_for_hook).unwrap();
        });
        let result = file.apply(Some(&digest(b"old")), Some(b"new"));
        assert!(result.is_err());
        assert_test_effect_hook_consumed();
        assert_eq!(outside.snapshot(), outside_before);
        restore_ancestor(&inside, &original);
    }
}

#[test]
fn root_swap_before_file_rename_keeps_overwrite_in_original_descriptor_tree() {
    use std::os::unix::fs::symlink;
    let fixture = JourneyFixture::new("descriptor-root-rename");
    let outside = Outside::new("root-rename");
    fs::create_dir(outside.0.join("inside")).unwrap();
    fs::write(outside.0.join("inside/value.bin"), b"outside").unwrap();
    let file = write_scoped(fixture.confined(), "inside/value.bin", b"old");
    let outside_before = outside.snapshot();
    let moved = renamed(&fixture.root, "descriptor-original");
    let root_for_hook = fixture.root.clone();
    let moved_for_hook = moved.clone();
    let outside_path = outside.0.clone();
    set_test_effect_hook(EffectPoint::Rename, move |_| {
        fs::rename(&root_for_hook, &moved_for_hook).unwrap();
        symlink(&outside_path, &root_for_hook).unwrap();
    });
    let error = file.apply(Some(&digest(b"old")), Some(b"new")).unwrap_err();
    assert_eq!(error.id(), ErrorId::ObjectChanged);
    assert_test_effect_hook_consumed();
    assert_eq!(outside.snapshot(), outside_before);
    restore_root(&fixture.root, &moved);
}

#[test]
fn root_swap_immediately_before_mkdir_does_not_create_outside_parent() {
    use std::os::unix::fs::symlink;
    let fixture = JourneyFixture::new("descriptor-root-mkdir");
    let outside = Outside::new("root-mkdir");
    fs::create_dir(outside.0.join("new-parent")).unwrap();
    fs::write(outside.0.join("new-parent/sentinel"), b"outside").unwrap();
    let outside_before = outside.snapshot();
    let file = ScopedFile::new(fixture.confined(), "new-parent/value.bin").unwrap();
    let moved = renamed(&fixture.root, "descriptor-original");
    let root_for_hook = fixture.root.clone();
    let moved_for_hook = moved.clone();
    let outside_path = outside.0.clone();
    set_test_effect_hook_matching(EffectPoint::Mkdir, "new-parent", move |_| {
        fs::rename(&root_for_hook, &moved_for_hook).unwrap();
        symlink(&outside_path, &root_for_hook).unwrap();
    });
    assert!(file.apply(None, Some(b"new")).is_err());
    assert_test_effect_hook_consumed();
    assert_eq!(outside.snapshot(), outside_before);
    restore_root(&fixture.root, &moved);
}

#[test]
fn ancestor_swap_immediately_before_open_is_zero_write_and_fails_closed() {
    use std::os::unix::fs::symlink;
    let fixture = JourneyFixture::new("descriptor-open");
    let outside = Outside::new("open");
    fs::create_dir(fixture.root.join("inside")).unwrap();
    fs::write(fixture.root.join("inside/value.bin"), b"inside").unwrap();
    fs::write(outside.0.join("value.bin"), b"outside").unwrap();
    let outside_before = outside.snapshot();
    let inside = fixture.root.join("inside");
    let original = fixture.root.join("inside-original");
    let inside_for_hook = inside.clone();
    let original_for_hook = original.clone();
    let outside_path = outside.0.clone();
    let file = ScopedFile::new(fixture.confined(), "inside/value.bin").unwrap();
    set_test_effect_hook_matching(EffectPoint::OpenFile, "inside/value.bin", move |_| {
        fs::rename(&inside_for_hook, &original_for_hook).unwrap();
        symlink(&outside_path, &inside_for_hook).unwrap();
    });
    assert!(file.inspect(64).is_err());
    assert_test_effect_hook_consumed();
    assert_eq!(outside.snapshot(), outside_before);
    restore_ancestor(&inside, &original);
}

// Imported from the independent ancestor_escape reproducer. The earlier
// candidate returned ObjectChanged only after rename_swap had written through
// a relocated descendant descriptor. This contract rejects at the mutation
// boundary: ObjectChanged is a pass here only when the outside bytes remain
// byte-identical.
#[test]
fn regular_ancestor_substitution_then_relocation_is_rejected_before_effect_boundary() {
    let fixture = JourneyFixture::new("ancestor-relocation");
    let outside = Outside::new("ancestor-relocation");
    fs::create_dir(fixture.root.join("inside")).unwrap();
    fs::write(fixture.root.join("inside/value.bin"), b"same-prior").unwrap();
    fs::write(outside.0.join("value.bin"), b"same-prior").unwrap();

    let scoped = ScopedFile::new(fixture.confined(), "inside/value.bin").unwrap();
    let original = fixture.root.join("inside-original");
    let root_inside = fixture.root.join("inside");
    let outside_before_effect = Rc::new(RefCell::new(None));
    let observed = Rc::clone(&outside_before_effect);
    let hook_root_inside = root_inside.clone();
    let hook_original = original.clone();
    let hook_outside = outside.0.clone();

    set_test_effect_hook_matching(EffectPoint::OpenDirectory, "inside", move |_| {
        fs::rename(&hook_root_inside, &hook_original).unwrap();
        fs::rename(&hook_outside, &hook_root_inside).unwrap();

        let rename_root_inside = hook_root_inside.clone();
        let rename_original = hook_original.clone();
        let rename_outside = hook_outside.clone();
        let rename_observed = Rc::clone(&observed);
        set_test_effect_hook(EffectPoint::Rename, move |_| {
            fs::rename(&rename_root_inside, &rename_outside).unwrap();
            fs::rename(&rename_original, &rename_root_inside).unwrap();
            rename_observed.replace(Some(fs::read(rename_outside.join("value.bin")).unwrap()));
        });
    });

    let error = scoped
        .apply(Some(&digest(b"same-prior")), Some(b"candidate"))
        .unwrap_err();
    assert_eq!(error.id(), ErrorId::ObjectChanged);
    assert_test_effect_hook_consumed();
    assert_eq!(
        outside_before_effect.borrow().as_deref(),
        Some(b"same-prior".as_slice())
    );
    assert_eq!(
        fs::read(outside.0.join("value.bin")).unwrap(),
        b"same-prior"
    );
    assert_eq!(
        fs::read(fixture.root.join("inside/value.bin")).unwrap(),
        b"same-prior"
    );
}

#[test]
fn directory_ancestor_substitution_then_relocation_preserves_outside_tree() {
    let fixture = JourneyFixture::new("tree-ancestor-relocation");
    let plan = fixture.plan();
    let target = "installed/tree";
    let mut scoped = ScopedTree::new(fixture.confined(), target).unwrap();
    materialize_package(&plan, &ExpectedTree::Absent, &mut scoped).unwrap();

    let outside = Outside::new("tree-ancestor-relocation");
    copy_regular_tree(&fixture.root.join("installed"), &outside.0);
    let outside_before = outside.snapshot();
    let installed = fixture.root.join("installed");
    let original = fixture.root.join("installed-original");
    let hook_installed = installed.clone();
    let hook_original = original.clone();
    let hook_outside = outside.0.clone();
    set_test_effect_hook_matching(EffectPoint::OpenDirectory, "installed", move |_| {
        fs::rename(&hook_installed, &hook_original).unwrap();
        fs::rename(&hook_outside, &hook_installed).unwrap();

        let rename_installed = hook_installed.clone();
        let rename_original = hook_original.clone();
        let rename_outside = hook_outside.clone();
        set_test_effect_hook(EffectPoint::Rename, move |_| {
            fs::rename(&rename_installed, &rename_outside).unwrap();
            fs::rename(&rename_original, &rename_installed).unwrap();
        });
    });

    assert!(
        materialize_package(
            &plan,
            &ExpectedTree::ExactDigest(plan.source_tree_sha256().into()),
            &mut scoped,
        )
        .is_err()
    );
    assert_test_effect_hook_consumed();
    assert_eq!(outside.snapshot(), outside_before);
}

#[test]
fn regular_ancestor_relocation_rejects_file_removal_before_fallback_rename() {
    let fixture = JourneyFixture::new("ancestor-relocation-removal");
    let outside = Outside::new("ancestor-relocation-removal");
    fs::create_dir(fixture.root.join("inside")).unwrap();
    fs::write(fixture.root.join("inside/value.bin"), b"same-prior").unwrap();
    fs::write(outside.0.join("value.bin"), b"same-prior").unwrap();

    let scoped = ScopedFile::new(fixture.confined(), "inside/value.bin").unwrap();
    let original = fixture.root.join("inside-original");
    let root_inside = fixture.root.join("inside");
    let outside_before_effect = Rc::new(RefCell::new(None));
    let observed = Rc::clone(&outside_before_effect);
    let hook_root_inside = root_inside.clone();
    let hook_original = original.clone();
    let hook_outside = outside.0.clone();
    set_test_effect_hook_matching(EffectPoint::OpenDirectory, "inside", move |_| {
        fs::rename(&hook_root_inside, &hook_original).unwrap();
        fs::rename(&hook_outside, &hook_root_inside).unwrap();

        let rename_root_inside = hook_root_inside.clone();
        let rename_original = hook_original.clone();
        let rename_outside = hook_outside.clone();
        let rename_observed = Rc::clone(&observed);
        set_test_effect_hook(EffectPoint::Rename, move |_| {
            fs::rename(&rename_root_inside, &rename_outside).unwrap();
            fs::rename(&rename_original, &rename_root_inside).unwrap();
            rename_observed.replace(Some(fs::read(rename_outside.join("value.bin")).unwrap()));
        });
    });

    let error = scoped
        .apply(Some(&digest(b"same-prior")), None)
        .unwrap_err();
    assert_eq!(error.id(), ErrorId::ObjectChanged);
    assert_test_effect_hook_consumed();
    assert_eq!(
        outside_before_effect.borrow().as_deref(),
        Some(b"same-prior".as_slice())
    );
    assert_eq!(
        fs::read(outside.0.join("value.bin")).unwrap(),
        b"same-prior"
    );
    assert_eq!(
        fs::read(fixture.root.join("inside/value.bin")).unwrap(),
        b"same-prior"
    );
}

#[test]
fn root_swap_immediately_before_unlink_cannot_delete_outside_collision() {
    use std::os::unix::fs::symlink;
    let fixture = JourneyFixture::new("descriptor-unlink");
    let outside = Outside::new("unlink");
    let file = write_scoped(fixture.confined(), "state/value.bin", b"old");
    let moved = renamed(&fixture.root, "descriptor-original");
    let root_for_hook = fixture.root.clone();
    let moved_for_hook = moved.clone();
    let outside_path = outside.0.clone();
    let outside_at_unlink = Rc::new(RefCell::new(None));
    let observed = outside_at_unlink.clone();
    set_test_effect_hook_matching(EffectPoint::Unlink, ".hul-quarantine-", move |detail| {
        fs::rename(&root_for_hook, &moved_for_hook).unwrap();
        symlink(&outside_path, &root_for_hook).unwrap();
        let leaf = detail.rsplit('/').next().unwrap();
        fs::write(outside_path.join(leaf), b"outside-stage").unwrap();
        observed.replace(Some(tree(&outside_path)));
    });
    assert!(file.apply(Some(&digest(b"old")), None).is_err());
    assert_test_effect_hook_consumed();
    assert_eq!(
        outside.snapshot(),
        outside_at_unlink.borrow().clone().unwrap(),
    );
    restore_root(&fixture.root, &moved);
}

#[test]
fn lock_and_stage_collisions_fail_without_overwrite_or_cleanup_outside() {
    use std::os::unix::fs::symlink;
    let fixture = JourneyFixture::new("descriptor-collisions");
    let outside = Outside::new("collisions");
    fs::write(outside.0.join("sentinel"), b"outside").unwrap();
    let relative = "state/value.bin";
    let file = ScopedFile::new(fixture.confined(), relative).unwrap();
    let lock = fixture
        .root
        .join(format!(".hul-lock-{}", &digest(relative.as_bytes())[7..]));
    fs::write(&lock, b"foreign-lock").unwrap();
    assert_eq!(
        file.apply(None, Some(b"value")).unwrap_err().id(),
        ErrorId::InstallConflict
    );
    assert_eq!(fs::read(&lock).unwrap(), b"foreign-lock");
    fs::remove_file(&lock).unwrap();

    let outside_before = outside.snapshot();
    let stage_path = Rc::new(RefCell::new(None));
    let recorded = stage_path.clone();
    let root = fixture.root.clone();
    let sentinel = outside.0.join("sentinel");
    set_test_effect_hook_matching(EffectPoint::CreateFile, ".hul-stage-", move |detail| {
        let path = root.join(detail.rsplit('/').next().unwrap());
        symlink(&sentinel, &path).unwrap();
        recorded.replace(Some(path));
    });
    assert_eq!(
        file.apply(None, Some(b"value")).unwrap_err().id(),
        ErrorId::EffectFailed
    );
    assert_test_effect_hook_consumed();
    assert_eq!(outside.snapshot(), outside_before);
    fs::remove_file(stage_path.borrow().as_ref().unwrap()).unwrap();
}

#[test]
fn parent_recreation_before_rename_is_not_treated_as_the_opened_parent() {
    let fixture = JourneyFixture::new("descriptor-parent-recreation");
    fs::create_dir(fixture.root.join("inside")).unwrap();
    let file = write_scoped(fixture.confined(), "inside/value.bin", b"old");
    let inside = fixture.root.join("inside");
    let original = fixture.root.join("inside-original");
    let recreated_at_effect = Rc::new(RefCell::new(None));
    let observed = recreated_at_effect.clone();
    let inside_for_hook = inside.clone();
    let original_for_hook = original.clone();
    set_test_effect_hook(EffectPoint::Rename, move |_| {
        fs::rename(&inside_for_hook, &original_for_hook).unwrap();
        fs::create_dir(&inside_for_hook).unwrap();
        fs::write(inside_for_hook.join("sentinel"), b"recreated").unwrap();
        observed.replace(Some(tree(&inside_for_hook)));
    });
    assert!(file.apply(Some(&digest(b"old")), Some(b"new")).is_err());
    assert_test_effect_hook_consumed();
    assert_eq!(tree(&inside), recreated_at_effect.borrow().clone().unwrap());
    fs::remove_dir_all(&inside).unwrap();
    fs::rename(&original, &inside).unwrap();
}

#[test]
fn tree_materialization_and_recovery_rename_races_preserve_outside_tree() {
    use std::os::unix::fs::symlink;
    let fixture = JourneyFixture::new("descriptor-tree-races");
    let outside = Outside::new("tree-races");
    fs::create_dir_all(outside.0.join("installed/tree")).unwrap();
    fs::write(outside.0.join("installed/tree/sentinel"), b"outside").unwrap();
    let plan = fixture.plan();
    let target = "installed/tree";
    let mut scoped = ScopedTree::new(fixture.confined(), target).unwrap();
    materialize_package(&plan, &ExpectedTree::Absent, &mut scoped).unwrap();
    let outside_before = outside.snapshot();
    let installed = fixture.root.join("installed");
    let original = fixture.root.join("installed-original");
    let installed_for_hook = installed.clone();
    let original_for_hook = original.clone();
    let outside_installed = outside.0.join("installed");
    set_test_effect_hook(EffectPoint::Rename, move |_| {
        fs::rename(&installed_for_hook, &original_for_hook).unwrap();
        symlink(&outside_installed, &installed_for_hook).unwrap();
    });
    let result = materialize_package(
        &plan,
        &ExpectedTree::ExactDigest(plan.source_tree_sha256().into()),
        &mut scoped,
    );
    assert!(result.is_err());
    assert_test_effect_hook_consumed();
    assert_eq!(outside.snapshot(), outside_before);
    restore_ancestor(&installed, &original);

    let token = &digest(target.as_bytes())[7..];
    let backup = fixture
        .root
        .join(format!(".hul-tree-{token}-backup-manual"));
    fs::rename(fixture.root.join(target), &backup).unwrap();
    let moved = renamed(&fixture.root, "descriptor-recovery-original");
    let root_for_hook = fixture.root.clone();
    let moved_for_hook = moved.clone();
    let outside_path = outside.0.clone();
    set_test_effect_hook(EffectPoint::Rename, move |_| {
        fs::rename(&root_for_hook, &moved_for_hook).unwrap();
        symlink(&outside_path, &root_for_hook).unwrap();
    });
    assert!(scoped.recover_interrupted().is_err());
    assert_test_effect_hook_consumed();
    assert_eq!(outside.snapshot(), outside_before);
    restore_root(&fixture.root, &moved);
}

fn restore_ancestor(current: &Path, original: &Path) {
    fs::remove_file(current).unwrap();
    fs::rename(original, current).unwrap();
}

fn restore_root(root: &Path, moved: &Path) {
    fs::remove_file(root).unwrap();
    fs::rename(moved, root).unwrap();
}

fn copy_regular_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let source = entry.path();
        let destination = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_regular_tree(&source, &destination);
        } else {
            fs::copy(&source, &destination).unwrap();
        }
    }
}
