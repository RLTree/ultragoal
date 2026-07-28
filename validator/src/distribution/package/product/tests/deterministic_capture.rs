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
    assert_eq!(first.snapshot().entries().len(), 19);
    assert_eq!(
        first
            .snapshot()
            .entries()
            .iter()
            .filter(|entry| entry.role() == PackageRole::Skill)
            .count(),
        8
    );
    for role in [PackageRole::Documentation] {
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
            .filter(|entry| entry.role() == PackageRole::Data)
            .count(),
        1
    );
    assert_eq!(
        first
            .snapshot()
            .entries()
            .iter()
            .filter(|entry| entry.role() == PackageRole::Executable)
            .count(),
        1
    );
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
    assert_eq!(package.snapshot().entries().len(), 19);
    assert!(
        package
            .snapshot()
            .entries()
            .iter()
            .all(|entry| !entry.path().starts_with("skills/legacy/"))
    );
}

#[test]
fn source_only_package_cannot_publish_an_installed_runtime() {
    let repo = Repo::new("supported-package-product-runtime");
    let context = repo.context();
    let artifact = capture_product_package(&context, &catalog(&context)).expect("package");
    let output = OutputRoot::new("supported-package-product-runtime");
    let confined = ConfinedRoot::open(&output.root).expect("confined root");
    let executable = ScopedFile::new(
        confined.clone(),
        "plugins/harness-ultragoal/runtime/ultragoal",
    )
    .unwrap();
    assert!(
        publish_installed_runtime_probe(artifact.snapshot(), &executable).is_err(),
        "a source-only package must not publish the retired static runtime probe"
    );
}

#[test]
fn verified_artifact_archive_publication_is_repeatable() {
    let repo = Repo::new("supported-package-product-archive");
    let context = repo.context();
    let catalog = catalog(&context);
    let artifact = capture_product_package(&context, &catalog).expect("package");
    let output = OutputRoot::new("supported-package-product-archive");
    let file = ScopedFile::new(
        ConfinedRoot::open(&output.root).expect("confined root"),
        "packages/harness-ultragoal.hugpkg",
    )
    .expect("archive output");
    artifact
        .publish_archive(&context, &catalog, &file)
        .expect("first publication");
    artifact
        .publish_archive(&context, &catalog, &file)
        .expect("repeat publication");
    assert_eq!(
        file.inspect(65 * 1024 * 1024).unwrap().unwrap(),
        artifact.snapshot().archive()
    );
}

#[test]
fn source_change_during_archive_publication_restores_prior_output() {
    let repo = Repo::new("supported-package-product-archive-rollback");
    let context = repo.workspace_context();
    let catalog = catalog(&context);
    let artifact = capture_product_package(&context, &catalog).expect("package");
    let output = ScopedFile::new(
        ConfinedRoot::open_workspace(&context).expect("workspace root"),
        "target/ultragoal/harness-ultragoal.hugpkg",
    )
    .expect("archive output");
    let prior = b"prior archive";
    assert!(output.apply(None, Some(prior)).expect("seed prior output"));
    let source = repo.root.join("skills/prove/SKILL.md");
    set_test_effect_hook_matching(EffectPoint::Rename, ".hul-stage-", move |_| {
        fs::write(
            source,
            "---\nname: prove\n---\nchanged during publication\n",
        )
        .expect("mutate source");
    });

    let error = artifact
        .publish_archive(&context, &catalog, &output)
        .expect_err("source mutation published archive");

    assert_eq!(error.id(), ProductionPackageErrorId::SourceUnavailable);
    assert_test_effect_hook_consumed();
    assert_eq!(
        output.inspect(65 * 1024 * 1024).unwrap(),
        Some(prior.to_vec())
    );
    assert!(
        WalkDir::new(&repo.root)
            .into_iter()
            .filter_map(Result::ok)
            .all(|entry| !entry.file_name().to_string_lossy().starts_with(".hul-")),
        "archive publication left transaction artifacts behind",
    );
}
use crate::distribution::publish_installed_runtime_probe;
