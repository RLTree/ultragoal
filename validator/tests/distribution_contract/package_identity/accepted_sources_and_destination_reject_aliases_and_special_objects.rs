#[cfg(unix)]
#[test]
fn accepted_sources_and_destination_reject_aliases_and_special_objects() {
    use std::os::unix::fs::symlink;
    let (fixture, inventory) = source_fixture("accepted-package-hardlink");
    fs::hard_link(
        fixture.root.join("skills/prove/SKILL.md"),
        fixture.root.join("prove-alias"),
    )
    .unwrap();
    assert_eq!(
        plan_package_from_inventory(&fixture.root, &inventory)
            .unwrap_err()
            .id(),
        ErrorId::UnsafeObject
    );
    let (fixture, inventory) = source_fixture("accepted-package-symlink");
    fs::remove_file(fixture.root.join("skills/prove/SKILL.md")).unwrap();
    symlink(
        "../harness-ultragoal/SKILL.md",
        fixture.root.join("skills/prove/SKILL.md"),
    )
    .unwrap();
    assert_eq!(
        plan_package_from_inventory(&fixture.root, &inventory)
            .unwrap_err()
            .id(),
        ErrorId::UnsafeObject
    );
    let (fixture, inventory) = source_fixture("accepted-package-destination-special");
    let plan = plan_package_from_inventory(&fixture.root, &inventory).unwrap();
    for (kind, links) in [
        (TreeObjectKind::Symlink, 1),
        (TreeObjectKind::RegularFile, 2),
        (TreeObjectKind::Special, 1),
    ] {
        let mut sink = TreeSink {
            objects: Some(vec![TreeObject::adversarial(
                "bad".into(),
                kind,
                0o644,
                links,
                Vec::new(),
            )]),
            ..TreeSink::default()
        };
        assert_eq!(
            materialize_package(
                &plan,
                &ExpectedTree::ExactDigest(digest(b"irrelevant")),
                &mut sink
            )
            .unwrap_err()
            .id(),
            ErrorId::UnsafeObject
        );
    }
}

#[test]
fn materialization_race_and_substitution_preserve_external_state() {
    let (fixture, inventory) = source_fixture("accepted-package-race");
    let plan = plan_package_from_inventory(&fixture.root, &inventory).unwrap();
    let concurrent = vec![TreeObject::regular("user".into(), 0o644, b"state".to_vec())];
    let mut race = TreeSink {
        race: Some(concurrent.clone()),
        ..TreeSink::default()
    };
    assert_eq!(
        materialize_package(&plan, &ExpectedTree::Absent, &mut race)
            .unwrap_err()
            .id(),
        ErrorId::InstallConflict
    );
    assert_eq!(race.objects, Some(concurrent));
    let mut corrupt = TreeSink {
        corrupt_after_write: true,
        ..TreeSink::default()
    };
    assert_eq!(
        materialize_package(&plan, &ExpectedTree::Absent, &mut corrupt)
            .unwrap_err()
            .id(),
        ErrorId::ArchiveMismatch
    );
}
