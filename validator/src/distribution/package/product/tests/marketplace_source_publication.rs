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
    assert_eq!(publication.context_id(), artifact.context_id());
    assert_eq!(publication.candidate_id(), artifact.candidate_id());
    assert_eq!(publication.catalog_id(), artifact.catalog_id());
    assert_eq!(publication.relative_path(), "plugins/harness-ultragoal");
    let materialized = tree
        .inspect(MARKETPLACE_SOURCE_ENTRIES, MARKETPLACE_SOURCE_BYTES)
        .expect("inspect marketplace source")
        .expect("materialized marketplace source");
    for role in crate::agent_roles::CANONICAL_AGENT_ROLES {
        let entry = materialized
            .iter()
            .find(|entry| entry.path() == role.manifest_path)
            .expect("root agent marketplace member");
        assert_eq!(entry.mode(), 0o644);
        assert_eq!(
            entry.bytes(),
            fs::read(repo.root.join(role.manifest_path)).expect("source root agent")
        );
    }
    artifact
        .verify_marketplace_source(&context, &catalog, &tree)
        .expect("verified marketplace source");
    let repeated = artifact
        .materialize_marketplace_source(&context, &catalog, &mut tree)
        .expect("repeat marketplace source");
    assert_eq!(repeated.tree_sha256(), publication.tree_sha256());
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

#[test]
fn marketplace_transition_binds_prior_digest_and_restores_exact_prior_tree() {
    let prior_repo = Repo::new("marketplace-transition-prior");
    let prior_context = prior_repo.context();
    let prior_catalog = catalog(&prior_context);
    let prior = capture_product_package(&prior_context, &prior_catalog).expect("prior package");

    let target_repo = Repo::new("marketplace-transition-target");
    fs::write(
        target_repo.root.join("skills/routine-work/SKILL.md"),
        "---\nname: routine-work\n---\ntarget transition bytes\n",
    )
    .expect("distinct target bytes");
    let target_context = target_repo.context();
    let target_catalog = catalog(&target_context);
    let target = capture_product_package(&target_context, &target_catalog).expect("target package");

    let output = OutputRoot::new("marketplace-transition");
    let confined = ConfinedRoot::open(&output.root).expect("confined root");
    let mut tree =
        ScopedTree::new(confined, "plugins/harness-ultragoal").expect("marketplace tree");
    let prior_observation = prior
        .materialize_marketplace_source(&prior_context, &prior_catalog, &mut tree)
        .expect("prior marketplace source");
    let prior_sha256 = prior_observation.tree_sha256().to_owned();
    assert_ne!(prior_sha256, target.snapshot().identity().tree_sha256());

    assert!(
        target
            .begin_marketplace_source_transition(
                &target_context,
                &target_catalog,
                &mut tree,
                Some(&format!("sha256:{}", "f".repeat(64))),
            )
            .is_err()
    );
    assert_eq!(
        tree_sha256(&tree.inspect(4096, 65 * 1024 * 1024).unwrap().unwrap()).unwrap(),
        prior_sha256
    );

    let (target_observation, transition) = target
        .begin_marketplace_source_transition(
            &target_context,
            &target_catalog,
            &mut tree,
            Some(&prior_sha256),
        )
        .expect("target transition");
    assert_eq!(
        target_observation.tree_sha256(),
        target.snapshot().identity().tree_sha256()
    );
    target
        .rollback_marketplace_source_transition(transition, &mut tree, &prior_sha256)
        .expect("exact prior rollback");
    assert_eq!(
        tree_sha256(&tree.inspect(4096, 65 * 1024 * 1024).unwrap().unwrap()).unwrap(),
        prior_sha256
    );
}

#[test]
fn marketplace_transition_withholds_rollback_after_postimage_substitution() {
    let prior_repo = Repo::new("marketplace-ambiguous-prior");
    let prior_context = prior_repo.context();
    let prior_catalog = catalog(&prior_context);
    let prior = capture_product_package(&prior_context, &prior_catalog).expect("prior package");
    let target_repo = Repo::new("marketplace-ambiguous-target");
    fs::write(
        target_repo.root.join("skills/routine-work/SKILL.md"),
        "---\nname: routine-work\n---\nambiguous target bytes\n",
    )
    .expect("distinct target bytes");
    let target_context = target_repo.context();
    let target_catalog = catalog(&target_context);
    let target = capture_product_package(&target_context, &target_catalog).expect("target package");
    let output = OutputRoot::new("marketplace-ambiguous");
    let confined = ConfinedRoot::open(&output.root).expect("confined root");
    let mut tree =
        ScopedTree::new(confined, "plugins/harness-ultragoal").expect("marketplace tree");
    let prior_sha256 = prior
        .materialize_marketplace_source(&prior_context, &prior_catalog, &mut tree)
        .unwrap()
        .tree_sha256()
        .to_owned();
    let (_, transition) = target
        .begin_marketplace_source_transition(
            &target_context,
            &target_catalog,
            &mut tree,
            Some(&prior_sha256),
        )
        .expect("target transition");
    fs::write(
        output
            .root
            .join("plugins/harness-ultragoal/.codex-plugin/plugin.json"),
        b"postimage substitution",
    )
    .expect("postimage substitution");
    assert!(
        target
            .rollback_marketplace_source_transition(transition, &mut tree, &prior_sha256)
            .is_err()
    );
    assert_ne!(
        tree_sha256(&tree.inspect(4096, 65 * 1024 * 1024).unwrap().unwrap()).unwrap(),
        prior_sha256
    );
}
