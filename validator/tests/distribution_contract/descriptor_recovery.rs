#![cfg(unix)]

use crate::distribution::{
    EffectPoint, ExpectedTree, ScopedTree, assert_test_effect_hook_consumed, materialize_package,
    set_test_effect_hook_matching,
};
use crate::journey_support::JourneyFixture;
use crate::support::{digest, tree};
use std::cell::RefCell;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_OUTSIDE: AtomicU64 = AtomicU64::new(0);

#[test]
fn tree_stage_content_mutation_is_rolled_back_before_backup_cleanup() {
    let fixture = JourneyFixture::new("descriptor-tree-candidate-cas");
    let plan = fixture.plan();
    let target = "installed/tree";
    let target_path = fixture.root.join(target);
    let mut scoped = ScopedTree::new(fixture.confined(), target).unwrap();
    materialize_package(&plan, &ExpectedTree::Absent, &mut scoped).unwrap();
    let before = tree(&target_path);
    let stage_path = Rc::new(RefCell::new(None));
    let recorded = stage_path.clone();
    let root = fixture.root.clone();
    set_test_effect_hook_matching(EffectPoint::Rename, "-stage-", move |detail| {
        let stage = rooted(&root, detail.split("->").next().unwrap());
        fs::write(
            stage.join(".codex-plugin/plugin.json"),
            b"attacker-stage-tree",
        )
        .unwrap();
        recorded.replace(Some(stage));
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
    assert_eq!(tree(&target_path), before);
    assert_eq!(
        fs::read(
            stage_path
                .borrow()
                .as_ref()
                .unwrap()
                .join(".codex-plugin/plugin.json")
        )
        .unwrap(),
        b"attacker-stage-tree"
    );
}

#[test]
fn absent_tree_stage_mutation_restores_absence_and_preserves_the_stage() {
    let fixture = JourneyFixture::new("descriptor-tree-absent-cas");
    let plan = fixture.plan();
    let target = "installed/tree";
    let mut scoped = ScopedTree::new(fixture.confined(), target).unwrap();
    let stage_path = Rc::new(RefCell::new(None));
    let recorded = stage_path.clone();
    let root = fixture.root.clone();
    set_test_effect_hook_matching(EffectPoint::Rename, "-stage-", move |detail| {
        let stage = rooted(&root, detail.split("->").next().unwrap());
        fs::write(
            stage.join(".codex-plugin/plugin.json"),
            b"attacker-absent-stage-tree",
        )
        .unwrap();
        recorded.replace(Some(stage));
    });
    assert!(materialize_package(&plan, &ExpectedTree::Absent, &mut scoped).is_err());
    assert_test_effect_hook_consumed();
    assert!(!fixture.root.join(target).exists());
    assert_eq!(
        fs::read(
            stage_path
                .borrow()
                .as_ref()
                .unwrap()
                .join(".codex-plugin/plugin.json")
        )
        .unwrap(),
        b"attacker-absent-stage-tree"
    );
}

#[test]
fn recovery_rejects_a_special_backup_without_following_or_moving_it() {
    let fixture = JourneyFixture::new("descriptor-recovery-special");
    let target = "installed/tree";
    let scoped = ScopedTree::new(fixture.confined(), target).unwrap();
    let outside = Outside::new("recovery-special");
    fs::write(outside.0.join("sentinel"), b"outside").unwrap();
    let outside_before = tree(&outside.0);
    let backup = backup_path(&fixture.root, target, "special");
    symlink(&outside.0, &backup).unwrap();

    assert!(scoped.recover_interrupted().is_err());
    assert!(!fixture.root.join(target).exists());
    assert!(
        fs::symlink_metadata(&backup)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(tree(&outside.0), outside_before);
}

#[test]
fn recovery_backup_leaf_substitution_is_preserved_and_never_installed() {
    let fixture = JourneyFixture::new("descriptor-recovery-leaf-cas");
    let plan = fixture.plan();
    let target = "installed/tree";
    let mut scoped = ScopedTree::new(fixture.confined(), target).unwrap();
    materialize_package(&plan, &ExpectedTree::Absent, &mut scoped).unwrap();
    let backup = backup_path(&fixture.root, target, "leaf-cas");
    fs::rename(fixture.root.join(target), &backup).unwrap();
    let saved = backup.with_file_name(format!(
        "{}-expected-saved",
        backup.file_name().unwrap().to_string_lossy()
    ));
    let outside = Outside::new("recovery-leaf-cas");
    fs::write(outside.0.join("sentinel"), b"outside").unwrap();
    let outside_before = tree(&outside.0);
    let backup_for_hook = backup.clone();
    let saved_for_hook = saved.clone();
    let outside_for_hook = outside.0.clone();
    let matcher = backup.file_name().unwrap().to_string_lossy().into_owned();
    set_test_effect_hook_matching(EffectPoint::Rename, &matcher, move |_| {
        fs::rename(&backup_for_hook, &saved_for_hook).unwrap();
        symlink(&outside_for_hook, &backup_for_hook).unwrap();
    });

    assert!(scoped.recover_interrupted().is_err());
    assert_test_effect_hook_consumed();
    assert!(!fixture.root.join(target).exists());
    assert!(
        fs::symlink_metadata(&backup)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(saved.join(".codex-plugin/plugin.json").is_file());
    assert_eq!(tree(&outside.0), outside_before);
}

fn backup_path(root: &Path, target: &str, suffix: &str) -> PathBuf {
    let token = &digest(target.as_bytes())[7..];
    root.join(format!(".hul-tree-{token}-backup-{suffix}"))
}

fn rooted(root: &Path, detail: &str) -> PathBuf {
    let mut components = detail.split('/');
    let _root_name = components.next().unwrap();
    components.fold(root.to_path_buf(), |path, component| path.join(component))
}

struct Outside(PathBuf);

impl Outside {
    fn new(label: &str) -> Self {
        let path = PathBuf::from("/tmp").join(format!(
            "hul-distribution-recovery-{label}-{}-{}-{}",
            std::process::id(),
            NEXT_OUTSIDE.fetch_add(1, Ordering::Relaxed),
            unique_time(),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
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
