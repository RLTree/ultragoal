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
    set_test_effect_hook_matching(EffectPoint::Rename, "", move |_| {
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
    set_test_effect_hook_matching(EffectPoint::Rename, "", move |_| {
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
