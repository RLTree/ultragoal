fn binding(snapshot: &crate::distribution::PackageSnapshot) -> PackageArtifactBinding {
    PackageArtifactBinding::issue(snapshot).unwrap()
}

fn journey(
    fixture: &JourneyFixture,
    snapshot: &crate::distribution::PackageSnapshot,
) -> JourneyBinding {
    let host = HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "package-output",
        None,
    )
    .unwrap();
    JourneyBinding::new(snapshot.identity().clone(), &host, "local-harness-plugins").unwrap()
}

fn output(fixture: &JourneyFixture) -> ScopedTree {
    ScopedTree::new(fixture.confined(), "repository/packages/harness-ultragoal").unwrap()
}

#[test]
fn writes_one_verified_candidate_bound_pair_and_reproduces_cleanly() {
    let first = JourneyFixture::new("package-output-first");
    let second = JourneyFixture::new("package-output-second");
    let one = first.build("build/one.hugpkg");
    let two = second.build("build/two.hugpkg");
    assert_eq!(one.archive(), two.archive());
    assert_eq!(one.inventory(), two.inventory());

    let mut left = output(&first);
    let mut right = output(&second);
    let left_tx = publish_package_artifact(
        &one,
        &binding(&one),
        &journey(&first, &one),
        &ExpectedTree::Absent,
        &mut left,
    )
    .unwrap();
    let right_tx = publish_package_artifact(
        &two,
        &binding(&two),
        &journey(&second, &two),
        &ExpectedTree::Absent,
        &mut right,
    )
    .unwrap();
    let left_rows = left.inspect(2, 65 * 1024 * 1024).unwrap().unwrap();
    let right_rows = right.inspect(2, 65 * 1024 * 1024).unwrap().unwrap();
    assert_eq!(left_rows, right_rows);
    assert_eq!(left_tx.output_tree_sha256(), right_tx.output_tree_sha256());
    reconcile_package_artifact(&one, &binding(&one), Some(&left_rows)).unwrap();
    assert_eq!(left_rows.len(), 2);
    assert!(left_rows.iter().any(|row| row.path().ends_with(".hugpkg")));
    assert!(
        left_rows
            .iter()
            .any(|row| row.path().ends_with(".inventory.json"))
    );
}

#[test]
fn mismatched_binding_partial_pair_and_extra_or_executable_rows_fail_without_writes() {
    let fixture = JourneyFixture::new("package-output-negative");
    let snapshot = fixture.build("build/one.hugpkg");
    let output = output(&fixture);
    let wrong = binding(&snapshot).substituted("context_id", &digest(b"wrong-context"));
    assert_eq!(
        reconcile_package_artifact(&snapshot, &wrong, None)
            .unwrap_err()
            .id(),
        ErrorId::ProvenanceMismatch
    );
    assert!(output.inspect(2, 65 * 1024 * 1024).unwrap().is_none());

    let mut rows = vec![crate::distribution::TreeObject::regular(
        "harness-ultragoal-0.0.11.hugpkg".into(),
        0o644,
        snapshot.archive().to_vec(),
    )];
    assert_eq!(
        reconcile_package_artifact(&snapshot, &binding(&snapshot), Some(&rows))
            .unwrap_err()
            .id(),
        ErrorId::ArchiveMismatch
    );
    rows.push(crate::distribution::TreeObject::regular(
        "unexpected.bin".into(),
        0o755,
        b"executable".to_vec(),
    ));
    assert_eq!(
        reconcile_package_artifact(&snapshot, &binding(&snapshot), Some(&rows))
            .unwrap_err()
            .id(),
        ErrorId::ArchiveMismatch
    );
}

#[test]
fn digest_matched_partial_pair_is_not_completed_or_replaced() {
    let fixture = JourneyFixture::new("package-output-partial-pair");
    let snapshot = fixture.build("build/one.hugpkg");
    let partial = vec![TreeObject::regular(
        "harness-ultragoal-0.0.11.hugpkg".into(),
        0o644,
        snapshot.archive().to_vec(),
    )];
    let partial_sha256 = tree_sha256(&partial).unwrap();
    let partial_dir = fixture.root.join("repository/packages/harness-ultragoal");
    fs::create_dir_all(&partial_dir).unwrap();
    fs::write(
        partial_dir.join("harness-ultragoal-0.0.11.hugpkg"),
        snapshot.archive(),
    )
    .unwrap();
    let mut output = output(&fixture);

    assert_eq!(
        publish_package_artifact(
            &snapshot,
            &binding(&snapshot),
            &journey(&fixture, &snapshot),
            &ExpectedTree::ExactDigest(partial_sha256),
            &mut output,
        )
        .unwrap_err()
        .id(),
        ErrorId::InstallConflict
    );
    assert_eq!(output.inspect(2, 65 * 1024 * 1024).unwrap(), Some(partial));
}
