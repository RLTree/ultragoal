#[test]
fn marketplace_source_is_exact_and_detects_post_publication_substitution() {
    let repo = Repo::new("marketplace-source-publication");
    let context = repo.context();
    let catalog = catalog(&context);
    let artifact = capture_product_package(&context, &catalog).expect("package");
    let output = OutputRoot::new("marketplace-source-publication");
    let confined = ConfinedRoot::open(&output.root).expect("confined root");
    let mut tree =
        ScopedTree::new(confined, "plugins/harness-ultragoal").expect("marketplace tree");

    let publication = artifact
        .materialize_marketplace_source(&context, &catalog, &mut tree)
        .expect("marketplace source");

    assert_eq!(
        publication.tree_sha256(),
        artifact.snapshot().identity().tree_sha256()
    );
    artifact
        .verify_marketplace_source(&context, &catalog, &tree)
        .expect("verified marketplace source");
    fs::write(
        output
            .root
            .join("plugins/harness-ultragoal/.codex-plugin/plugin.json"),
        b"substituted",
    )
    .expect("substitute package member");
    assert!(
        artifact
            .verify_marketplace_source(&context, &catalog, &tree)
            .is_err()
    );
}

#[test]
fn marketplace_source_rejects_noncanonical_target() {
    let repo = Repo::new("marketplace-source-target");
    let context = repo.context();
    let catalog = catalog(&context);
    let artifact = capture_product_package(&context, &catalog).expect("package");
    let output = OutputRoot::new("marketplace-source-target");
    let confined = ConfinedRoot::open(&output.root).expect("confined root");
    let mut tree =
        ScopedTree::new(confined, "repository/packages/harness-ultragoal").expect("wrong tree");

    assert!(
        artifact
            .materialize_marketplace_source(&context, &catalog, &mut tree)
            .is_err()
    );
    assert!(!output.root.join("repository").exists());
}
