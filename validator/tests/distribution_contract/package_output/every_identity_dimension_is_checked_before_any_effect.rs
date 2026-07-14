#[test]
fn every_identity_dimension_is_checked_before_any_effect() {
    let fixture = JourneyFixture::new("package-output-full-identity");
    let snapshot = fixture.build("build/one.hugpkg");
    let permit = binding(&snapshot);
    for (dimension, replacement) in [
        ("context_id", digest(b"wrong-context")),
        ("candidate_id", digest(b"wrong-candidate")),
        ("plugin_id", "wrong-plugin".into()),
        ("version", "0.0.12".into()),
        ("catalog_id", digest(b"wrong-catalog")),
        (
            "accepted_inventory_sha256",
            digest(b"wrong-accepted-inventory"),
        ),
        ("source_tree_sha256", digest(b"wrong-source-tree")),
        ("package_sha256", digest(b"wrong-package")),
        ("inventory_sha256", digest(b"wrong-inventory")),
    ] {
        let mut effects = EffectCounter::default();
        let altered = permit.substituted(dimension, &replacement);
        assert_eq!(
            publish_package_artifact(&snapshot, &altered, &ExpectedTree::Absent, &mut effects)
                .unwrap_err()
                .id(),
            ErrorId::ProvenanceMismatch,
            "{dimension}"
        );
        assert_eq!((effects.reads, effects.transitions), (0, 0), "{dimension}");
    }
}

#[test]
fn verification_mismatch_rolls_back_the_exact_prior_pair() {
    let fixture = JourneyFixture::new("package-output-rollback");
    let snapshot = fixture.build("build/one.hugpkg");
    let binding = binding(&snapshot);
    let mut effects = CorruptAfterWrite::default();
    let prior =
        publish_package_artifact(&snapshot, &binding, &ExpectedTree::Absent, &mut effects).unwrap();
    let before = effects.rows.clone();
    effects.corrupt_on_read = effects.reads + 2;
    assert_eq!(
        publish_package_artifact(
            &snapshot,
            &binding,
            &ExpectedTree::ExactDigest(prior.output_tree_sha256().into()),
            &mut effects,
        )
        .unwrap_err()
        .id(),
        ErrorId::ArchiveMismatch
    );
    assert_eq!(effects.rows, before);
}

#[test]
fn rollback_and_descriptor_recovery_preserve_the_confined_output_pair() {
    let fixture = JourneyFixture::new("package-output-recovery");
    let snapshot = fixture.build("build/one.hugpkg");
    let binding = binding(&snapshot);
    let mut output = ScopedTree::new(fixture.confined(), "output/candidate").unwrap();
    let transaction =
        publish_package_artifact(&snapshot, &binding, &ExpectedTree::Absent, &mut output).unwrap();
    rollback_package_artifact(transaction, &mut output).unwrap();
    assert!(output.inspect(2, 65 * 1024 * 1024).unwrap().is_none());

    let token = &digest(b"output/candidate")[7..];
    let backup = fixture
        .root
        .join(format!(".hul-tree-{token}-backup-interrupted"));
    fs::create_dir(&backup).unwrap();
    fs::write(
        backup.join("harness-ultragoal-0.0.11.hugpkg"),
        snapshot.archive(),
    )
    .unwrap();
    fs::write(
        backup.join("harness-ultragoal-0.0.11.inventory.json"),
        snapshot.inventory(),
    )
    .unwrap();
    assert!(recover_package_artifact(&output).unwrap());
    let recovered = output.inspect(2, 65 * 1024 * 1024).unwrap().unwrap();
    reconcile_package_artifact(&snapshot, &binding, Some(&recovered)).unwrap();
}
