#[test]
fn every_identity_dimension_is_checked_before_any_effect() {
    let fixture = JourneyFixture::new("package-output-full-identity");
    let snapshot = fixture.build("build/one.hugpkg");
    let permit = binding(&snapshot);
    let journey = journey(&fixture, &snapshot);
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
        let mut output = output(&fixture);
        let altered = permit.substituted(dimension, &replacement);
        assert_eq!(
            publish_package_artifact(
                &snapshot,
                &altered,
                &journey,
                &ExpectedTree::Absent,
                &mut output,
            )
            .unwrap_err()
            .id(),
            ErrorId::ProvenanceMismatch,
            "{dimension}"
        );
        assert!(output.inspect(2, 65 * 1024 * 1024).unwrap().is_none());
    }
}

#[test]
fn same_package_cross_journey_publication_cannot_be_transplanted() {
    let fixture = JourneyFixture::new("package-output-transplant");
    let snapshot = fixture.build("build/one.hugpkg");
    let permit = binding(&snapshot);
    let journey = journey(&fixture, &snapshot);
    let mut output = output(&fixture);
    let transaction = publish_package_artifact(
        &snapshot,
        &permit,
        &journey,
        &ExpectedTree::Absent,
        &mut output,
    )
    .unwrap();
    let other_fixture = JourneyFixture::new("package-output-other-root");
    let other_host = HostCapabilityDeclaration::isolated(
        &other_fixture.root,
        &other_fixture.project,
        "package-output",
        None,
    )
    .unwrap();
    let other_journey = JourneyBinding::new(
        snapshot.identity().clone(),
        &other_host,
        "local-harness-plugins",
    )
    .unwrap();
    assert_eq!(
        SurfaceIdentity::from_published_package(&snapshot, &transaction, &other_journey)
            .unwrap_err()
            .id(),
        ErrorId::ProvenanceMismatch
    );
}

#[test]
fn rollback_and_descriptor_recovery_preserve_the_confined_output_pair() {
    let fixture = JourneyFixture::new("package-output-recovery");
    let snapshot = fixture.build("build/one.hugpkg");
    let binding = binding(&snapshot);
    let journey = journey(&fixture, &snapshot);
    let mut output = output(&fixture);
    let transaction = publish_package_artifact(
        &snapshot,
        &binding,
        &journey,
        &ExpectedTree::Absent,
        &mut output,
    )
    .unwrap();
    rollback_package_artifact(transaction, &journey, &mut output).unwrap();
    assert!(output.inspect(2, 65 * 1024 * 1024).unwrap().is_none());

    let token = &digest(b"repository/packages/harness-ultragoal")[7..];
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
