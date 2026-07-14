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
        set_test_effect_hook_matching(EffectPoint::Rename, "", move |_| {
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
        set_test_effect_hook_matching(EffectPoint::Rename, "", move |_| {
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
    set_test_effect_hook_matching(EffectPoint::Rename, "", move |_| {
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
