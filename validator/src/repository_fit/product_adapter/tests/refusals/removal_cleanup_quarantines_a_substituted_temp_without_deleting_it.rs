use super::*;

#[test]
pub(crate) fn removal_cleanup_quarantines_a_substituted_temp_without_deleting_it() {
    let fixture = Fixture::new("removal-cleanup-race");
    fixture.write("AGENTS.md", b"prior");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    effects.pause_before_quarantine_move(reached.clone(), resume.clone());
    let root = fixture.root.clone();
    let racer = std::thread::spawn(move || {
        reached.wait();
        let temp = shared_temp_file(&root);
        fs::rename(&temp, root.join("prior-moved")).unwrap();
        fs::write(&temp, b"foreign").unwrap();
        resume.wait();
    });
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("AGENTS.md").unwrap(),
            &ExpectedContent::ExactDigest(digest(b"prior")),
            None,
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert!(!fixture.root.join("AGENTS.md").exists());
    assert_eq!(
        fs::read(fixture.root.join("prior-moved")).unwrap(),
        b"prior"
    );
    assert_eq!(
        quarantined_objects(&fixture.root),
        vec![b"foreign".to_vec()]
    );
}

#[test]
pub(crate) fn staged_parent_publish_refuses_a_substituted_directory_without_writing_into_it() {
    let fixture = Fixture::new("parent-publish-race");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    effects.pause_after_directory_publish(reached.clone(), resume.clone());
    let root = fixture.root.clone();
    let racer = std::thread::spawn(move || {
        reached.wait();
        fs::rename(root.join("nested"), root.join("managed-moved")).unwrap();
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/marker"), b"foreign").unwrap();
        resume.wait();
    });
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("nested/AGENTS.md").unwrap(),
            &ExpectedContent::Absent,
            Some(b"replacement"),
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert_eq!(
        fs::read(fixture.root.join("nested/marker")).unwrap(),
        b"foreign"
    );
    assert!(fixture.root.join("managed-moved").is_dir());
    assert!(!fixture.root.join("nested/AGENTS.md").exists());
}

#[test]
pub(crate) fn created_parent_cleanup_quarantines_a_substituted_directory_without_deleting_it() {
    let fixture = Fixture::new("parent-cleanup-race");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    assert!(
        effects
            .compare_exchange(
                &CanonicalPath::parse("nested/AGENTS.md").unwrap(),
                &ExpectedContent::Absent,
                Some(b"managed"),
            )
            .unwrap()
    );
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    effects.pause_before_directory_quarantine_move(reached.clone(), resume.clone());
    let root = fixture.root.clone();
    let racer = std::thread::spawn(move || {
        reached.wait();
        fs::rename(root.join("nested"), root.join("managed-moved")).unwrap();
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/marker"), b"foreign").unwrap();
        resume.wait();
    });
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("nested/AGENTS.md").unwrap(),
            &ExpectedContent::ExactDigest(digest(b"managed")),
            None,
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert!(fixture.root.join("managed-moved").is_dir());
    assert_eq!(
        quarantined_directory_markers(&fixture.root),
        vec![b"foreign".to_vec()]
    );
}

pub(crate) fn shared_temp_file(root: &std::path::Path) -> std::path::PathBuf {
    fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .find(|entry| {
            entry.file_type().unwrap().is_file()
                && entry.file_name().to_string_lossy().starts_with(".hul-fit-")
        })
        .unwrap()
        .path()
}

pub(crate) fn quarantined_objects(root: &std::path::Path) -> Vec<Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| {
            entry.file_type().unwrap().is_dir()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".hul-fit-transaction-")
        })
        .map(|entry| fs::read(entry.path().join("object")).unwrap())
        .collect()
}

pub(crate) fn quarantined_directory_markers(root: &std::path::Path) -> Vec<Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| {
            entry.file_type().unwrap().is_dir()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".hul-fit-transaction-")
        })
        .filter_map(|entry| fs::read(entry.path().join("object/marker")).ok())
        .collect()
}

#[test]
pub(crate) fn cross_device_guard_fails_closed_before_any_namespace_effect() {
    assert!(crate::repository_fit::local::LocalEffects::test_device_guard(42, 42).is_ok());
    assert_eq!(
        crate::repository_fit::local::LocalEffects::test_device_guard(42, 43)
            .unwrap_err()
            .id(),
        FitErrorId::UnsafeObject
    );
}

#[test]
pub(crate) fn production_local_effects_require_a_root_owned_exclusive_mutation_lease() {
    let fixture = Fixture::new("missing-exclusive-mutation-lease");
    fixture.write("AGENTS.md", b"prior");
    let before = snapshot(&fixture.root);
    let mut effects =
        crate::repository_fit::local::LocalEffects::open(&fixture.root, modes()).unwrap();
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("AGENTS.md").unwrap(),
            &ExpectedContent::ExactDigest(digest(b"prior")),
            Some(b"replacement"),
        )
        .unwrap_err();
    assert_eq!(failure.id(), FitErrorId::Unauthorized);
    assert_eq!(snapshot(&fixture.root), before);
}
