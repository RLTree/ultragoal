use super::*;

#[test]
pub(crate) fn leaf_path_swap_and_stale_expected_digest_preserve_foreign_bytes() {
    let fixture = Fixture::new("leaf-swap");
    fixture.write("AGENTS.md", b"observed");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    fs::rename(
        fixture.root.join("AGENTS.md"),
        fixture.root.join("observed-moved"),
    )
    .unwrap();
    fs::write(fixture.root.join("AGENTS.md"), b"foreign").unwrap();
    assert!(
        !effects
            .compare_exchange(
                &CanonicalPath::parse("AGENTS.md").unwrap(),
                &ExpectedContent::ExactDigest(digest(b"observed")),
                Some(b"replacement"),
            )
            .unwrap()
    );
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        fs::read(fixture.root.join("observed-moved")).unwrap(),
        b"observed"
    );
}

#[test]
pub(crate) fn concurrent_leaf_swap_at_the_atomic_boundary_preserves_all_objects() {
    let fixture = Fixture::new("leaf-linearization-race");
    fixture.write("AGENTS.md", b"observed");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    effects.pause_before_linearize(reached.clone(), resume.clone());
    let root = fixture.root.clone();
    let racer = std::thread::spawn(move || {
        reached.wait();
        fs::rename(root.join("AGENTS.md"), root.join("observed-moved")).unwrap();
        fs::write(root.join("AGENTS.md"), b"foreign").unwrap();
        resume.wait();
    });
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("AGENTS.md").unwrap(),
            &ExpectedContent::ExactDigest(digest(b"observed")),
            Some(b"replacement"),
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        b"replacement"
    );
    assert_eq!(
        fs::read(fixture.root.join("observed-moved")).unwrap(),
        b"observed"
    );
    let preserved = fs::read_dir(&fixture.root)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| {
            entry.file_type().unwrap().is_file()
                && entry.file_name().to_string_lossy().starts_with(".hul-fit-")
        })
        .map(|entry| fs::read(entry.path()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(preserved, vec![b"foreign".to_vec()]);
}

#[test]
pub(crate) fn post_swap_identity_race_preserves_foreign_prior_and_replacement_objects() {
    let fixture = Fixture::new("post-swap-identity-race");
    fixture.write("AGENTS.md", b"prior");
    let mut effects =
        crate::repository_fit::local::LocalEffects::open_for_test(&fixture.root, modes()).unwrap();
    let reached = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    effects.pause_after_replace_swap(reached.clone(), resume.clone());
    let root = fixture.root.clone();
    let racer = std::thread::spawn(move || {
        reached.wait();
        fs::rename(root.join("AGENTS.md"), root.join("replacement-moved")).unwrap();
        fs::write(root.join("AGENTS.md"), b"foreign").unwrap();
        resume.wait();
    });
    let failure = effects
        .compare_exchange(
            &CanonicalPath::parse("AGENTS.md").unwrap(),
            &ExpectedContent::ExactDigest(digest(b"prior")),
            Some(b"replacement"),
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        fs::read(fixture.root.join("replacement-moved")).unwrap(),
        b"replacement"
    );
    let preserved = fs::read_dir(&fixture.root)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(".hul-fit-"))
        .map(|entry| fs::read(entry.path()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(preserved, vec![b"prior".to_vec()]);
}

#[test]
pub(crate) fn replacement_cleanup_quarantines_a_substituted_temp_without_deleting_it() {
    let fixture = Fixture::new("replacement-cleanup-race");
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
            Some(b"replacement"),
        )
        .err()
        .unwrap();
    racer.join().unwrap();
    assert_eq!(failure.id(), FitErrorId::RollbackFailed);
    assert_eq!(
        fs::read(fixture.root.join("AGENTS.md")).unwrap(),
        b"replacement"
    );
    assert_eq!(
        fs::read(fixture.root.join("prior-moved")).unwrap(),
        b"prior"
    );
    assert_eq!(
        quarantined_objects(&fixture.root),
        vec![b"foreign".to_vec()]
    );
}
