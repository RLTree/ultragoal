#[test]
fn package_inventory_publication_is_exact_and_repeatable() {
    let repo = Repo::new("package-inventory-publication");
    let context = repo.workspace_context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    let output = ScopedFile::new(
        ConfinedRoot::open_workspace(&context).expect("workspace root"),
        "target/ultragoal/package-inventory.json",
    )
    .expect("inventory output");

    artifact
        .publish_inventory(&context, &authority_catalog, &output)
        .expect("first inventory publication");
    let first = output
        .inspect(16 * 1024 * 1024)
        .expect("inventory inspection")
        .expect("published inventory");
    assert_eq!(first, artifact.snapshot().inventory());

    artifact
        .publish_inventory(&context, &authority_catalog, &output)
        .expect("repeat inventory publication");
    assert_eq!(
        output.inspect(16 * 1024 * 1024).unwrap(),
        Some(first),
        "repeat use changed exact inventory bytes",
    );
}

#[test]
fn source_change_during_inventory_publication_restores_prior_output() {
    let repo = Repo::new("package-inventory-publication-rollback");
    let context = repo.workspace_context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    let output = ScopedFile::new(
        ConfinedRoot::open_workspace(&context).expect("workspace root"),
        "target/ultragoal/package-inventory.json",
    )
    .expect("inventory output");
    let prior = b"{\"prior\":true}\n";
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
        .publish_inventory(&context, &authority_catalog, &output)
        .expect_err("source mutation published inventory");

    assert_eq!(error.id(), ProductionPackageErrorId::SourceUnavailable);
    assert_test_effect_hook_consumed();
    assert_eq!(
        output.inspect(16 * 1024 * 1024).unwrap(),
        Some(prior.to_vec()),
        "failed publication did not restore prior output",
    );
    assert!(
        WalkDir::new(&repo.root)
            .into_iter()
            .filter_map(Result::ok)
            .all(|entry| !entry.file_name().to_string_lossy().starts_with(".hul-")),
        "publication left transaction artifacts behind",
    );
}
