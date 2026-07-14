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
        set_test_effect_hook_matching(EffectPoint::Rename, "", move |_| {
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
    set_test_effect_hook_matching(EffectPoint::Rename, "", move |_| {
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
        set_test_effect_hook_matching(EffectPoint::Rename, "", move |_| {
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
