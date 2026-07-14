#[test]
fn independent_product_captures_are_byte_identical_and_reverified() {
    let repo = Repo::new("supported-package-product-identical");
    let context = repo.context();
    let catalog = catalog(&context);
    let before = status(&repo.root);
    let before_tree = source_tree(&repo.root);

    let first = capture_product_package(&context, &catalog).expect("first package");
    let second = capture_product_package(&context, &catalog).expect("second package");
    assert_eq!(first.snapshot().archive(), second.snapshot().archive());
    assert_eq!(first.snapshot().inventory(), second.snapshot().inventory());
    assert_eq!(first.source_inventory(), second.source_inventory());
    assert_eq!(first.snapshot().entries().len(), 17);
    assert_eq!(
        first
            .snapshot()
            .entries()
            .iter()
            .filter(|entry| entry.role() == PackageRole::Skill)
            .count(),
        8
    );
    for role in [
        PackageRole::Documentation,
        PackageRole::Executable,
        PackageRole::Data,
    ] {
        assert_eq!(
            first
                .snapshot()
                .entries()
                .iter()
                .filter(|entry| entry.role() == role)
                .count(),
            0,
            "noncanonical package role became active: {role:?}"
        );
    }
    assert_eq!(
        first
            .snapshot()
            .entries()
            .iter()
            .filter(|entry| entry.role() == PackageRole::Agent)
            .count(),
        8
    );
    verify_product_package(&first, &context, &catalog).expect("independent verify");
    assert_eq!(status(&repo.root), before);
    assert_eq!(source_tree(&repo.root), before_tree);
}

#[test]
fn legacy_skill_source_is_not_active_package_membership() {
    let repo = Repo::new("supported-package-product-legacy-excluded");
    fs::create_dir_all(repo.root.join("skills/legacy/agents")).expect("legacy metadata");
    fs::write(repo.root.join("skills/legacy/SKILL.md"), "legacy\n").expect("legacy skill");
    fs::write(
        repo.root.join("skills/legacy/agents/openai.yaml"),
        "interface: {}\n",
    )
    .expect("legacy metadata");
    let context = repo.context();
    let catalog = catalog(&context);
    let package = capture_product_package(&context, &catalog).expect("package");
    assert_eq!(package.snapshot().entries().len(), 17);
    assert!(
        package
            .snapshot()
            .entries()
            .iter()
            .all(|entry| !entry.path().starts_with("skills/legacy/"))
    );
}
