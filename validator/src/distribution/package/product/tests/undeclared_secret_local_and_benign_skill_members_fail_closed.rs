#[test]
fn undeclared_secret_local_and_benign_skill_members_fail_closed() {
    for (label, relative, bytes) in [
        (
            "secret",
            "skills/prove/.env",
            b"TOKEN=secret-package-canary\n".as_slice(),
        ),
        (
            "local",
            "skills/prove/.DS_Store",
            b"local-only\n".as_slice(),
        ),
        (
            "benign",
            "skills/prove/notes.txt",
            b"undeclared\n".as_slice(),
        ),
    ] {
        let repo = Repo::new(&format!("supported-package-product-unknown-{label}"));
        fs::write(repo.root.join(relative), bytes).expect("undeclared member");
        let context = repo.context();
        let error = capture_product_package(&context, &catalog(&context))
            .expect_err("undeclared package member accepted");
        assert_eq!(error.id(), ProductionPackageErrorId::SourceUnavailable);
        let diagnostic = error.to_string();
        assert!(!diagnostic.contains(relative));
        assert!(!diagnostic.contains("secret-package-canary"));
    }
}

#[test]
fn missing_or_unsafe_canonical_agent_member_fails_closed() {
    let missing = Repo::new("supported-package-product-missing-agent");
    fs::remove_file(missing.root.join("skills/prove/agents/openai.yaml"))
        .expect("remove canonical agent");
    let context = missing.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("missing canonical agent accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let linked = Repo::new("supported-package-product-linked-agent");
    fs::hard_link(
        linked.root.join("skills/prove/agents/openai.yaml"),
        linked.root.join("skills/prove/agents/duplicate.yaml"),
    )
    .expect("hard-linked agent");
    let context = linked.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("hard-linked canonical agent accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );
}

#[test]
fn manifest_absence_version_drift_and_mutate_restore_fail_closed() {
    let repo = Repo::new("supported-package-product-version-drift");
    write_draft(&repo.root, "0.0.14");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let error =
        capture_product_package(&context, &authority_catalog).expect_err("version drift accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::ManifestMismatch);

    write_draft(&repo.root, SUPPORTED_VERSION);
    write_supported_manifest(&repo.root, "0.0.14");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let error = capture_product_package(&context, &authority_catalog)
        .expect_err("supported manifest version drift accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::ManifestMismatch);

    write_supported_manifest(&repo.root, SUPPORTED_VERSION);
    fs::remove_file(repo.root.join(SUPPORTED_MANIFEST_PATH)).expect("remove supported manifest");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let error = capture_product_package(&context, &authority_catalog)
        .expect_err("missing supported manifest accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::SourceUnavailable);

    write_supported_manifest(&repo.root, SUPPORTED_VERSION);
    fs::remove_file(repo.root.join("plugin-manifest-draft.json")).expect("remove draft manifest");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let error = capture_product_package(&context, &authority_catalog)
        .expect_err("missing draft manifest accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::SourceUnavailable);

    write_draft(&repo.root, SUPPORTED_VERSION);
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let session = ProductionPackageSession::begin(&context, &authority_catalog).expect("session");
    let path = repo.root.join("skills/prove/SKILL.md");
    let original = fs::read(&path).expect("original skill");
    fs::write(&path, "mutated\n").expect("mutate skill");
    fs::write(&path, original).expect("restore skill");
    let error = session.finish().expect_err("mutate restore accepted");
    assert!(matches!(
        error.id(),
        ProductionPackageErrorId::SourceUnavailable | ProductionPackageErrorId::ContextUnavailable
    ));
}

#[test]
fn publication_is_current_bound_and_reconciles_one_complete_pair() {
    let repo = Repo::new("supported-package-product-publication");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let before = status(&repo.root);
    let before_tree = source_tree(&repo.root);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    let output_root = OutputRoot::new("supported-package-product-publication");
    let mut output = output_root.tree();

    let transaction = artifact
        .publish(
            &context,
            &authority_catalog,
            &output_root.journey(&artifact),
            &ExpectedTree::Absent,
            &mut output,
        )
        .expect("current-bound publication");
    let rows = output
        .inspect(2, 65 * 1024 * 1024)
        .expect("inspect package output")
        .expect("published artifact pair");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        tree_sha256(&rows).unwrap(),
        transaction.output_tree_sha256()
    );
    assert!(rows.iter().any(|row| row.path().ends_with(".hugpkg")));
    assert!(
        rows.iter()
            .any(|row| row.path().ends_with(".inventory.json"))
    );
    let identity = SurfaceIdentity::from_published_package(
        artifact.snapshot(),
        &transaction,
        &output_root.journey(&artifact),
    )
    .expect("published package identity");
    assert_eq!(
        identity.observation_sha256(),
        transaction.output_tree_sha256()
    );
    verify_product_package(&artifact, &context, &authority_catalog).expect("post-publish verify");
    assert_eq!(status(&repo.root), before);
    assert_eq!(source_tree(&repo.root), before_tree);
}

#[test]
fn stale_candidate_or_forged_catalog_is_rejected_before_output_effects() {
    let repo = Repo::new("supported-package-product-stale-publication");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    fs::write(repo.root.join("skills/prove/SKILL.md"), "changed\n").expect("candidate drift");
    let changed_context = repo.context();
    let changed_catalog = catalog(&changed_context);
    let output_root = OutputRoot::new("supported-package-product-stale-publication");
    let mut output = output_root.tree();
    let error = artifact
        .publish(
            &changed_context,
            &changed_catalog,
            &output_root.journey(&artifact),
            &ExpectedTree::Absent,
            &mut output,
        )
        .expect_err("stale artifact published");
    assert_eq!(error.id(), ProductionPackageErrorId::CatalogMismatch);
    assert!(output.inspect(2, 65 * 1024 * 1024).unwrap().is_none());

    let current_artifact =
        capture_product_package(&changed_context, &changed_catalog).expect("current package");
    let forged_catalog = AuthorityCatalog::new(AuthorityCatalogDefinition {
        catalog_id: format!("sha256:{}", "f".repeat(64)),
        context_id: changed_context.context_id().to_owned(),
        contract_id: "test-contract".to_owned(),
        source_registry_counts: BTreeMap::new(),
        entries: Vec::new(),
        findings: Vec::new(),
        generated_surfaces: GeneratedSurfaceIndex::new(Vec::new()),
    });
    let error = current_artifact
        .publish(
            &changed_context,
            &forged_catalog,
            &output_root.journey(&current_artifact),
            &ExpectedTree::Absent,
            &mut output,
        )
        .expect_err("forged catalog published");
    assert_eq!(error.id(), ProductionPackageErrorId::CatalogMismatch);
    assert!(output.inspect(2, 65 * 1024 * 1024).unwrap().is_none());
}
