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
fn missing_or_unsafe_canonical_skill_agent_metadata_fails_closed() {
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
    let context = linked.context();
    fs::hard_link(
        linked.root.join("skills/prove/agents/openai.yaml"),
        linked.root.join("skills/prove/agents/duplicate.yaml"),
    )
    .expect("hard-linked agent");
    assert!(matches!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("hard-linked canonical agent accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable | ProductionPackageErrorId::ContextUnavailable
    ));
}

#[test]
fn root_agent_membership_rejects_missing_unknown_case_colliding_and_unsafe_members() {
    use std::os::unix::ffi::OsStrExt;

    let canonical = crate::agent_roles::CANONICAL_AGENT_ROLES[0].manifest_path;

    let missing = Repo::new("supported-package-product-missing-root-agent");
    fs::remove_file(missing.root.join(canonical)).expect("remove root agent");
    let context = missing.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("missing root agent accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let unknown = Repo::new("supported-package-product-unknown-root-agent");
    fs::write(
        unknown.root.join(".codex/agents/undeclared.toml"),
        "name = \"undeclared\"\n",
    )
    .expect("unknown root agent");
    let context = unknown.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("unknown root agent accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let case_colliding = Repo::new("supported-package-product-case-root-agent");
    fs::rename(
        case_colliding.root.join(canonical),
        case_colliding
            .root
            .join(".codex/agents/Claim-Falsifier.toml"),
    )
    .expect("case-colliding root agent");
    let context = case_colliding.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("case-colliding root agent accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let linked = Repo::new("supported-package-product-hard-linked-root-agent");
    let context = linked.context();
    fs::hard_link(
        linked.root.join(canonical),
        linked.root.join("agent-link-alias"),
    )
    .expect("hard-link root agent");
    assert!(matches!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("hard-linked root agent accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable | ProductionPackageErrorId::ContextUnavailable
    ));

    let symlinked = Repo::new("supported-package-product-symlinked-root-agent");
    let context = symlinked.context();
    fs::remove_file(symlinked.root.join(canonical)).expect("remove root agent for symlink");
    symlink(
        symlinked
            .root
            .join(crate::agent_roles::CANONICAL_AGENT_ROLES[1].manifest_path),
        symlinked.root.join(canonical),
    )
    .expect("symlink root agent");
    assert!(matches!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("symlinked root agent accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable | ProductionPackageErrorId::ContextUnavailable
    ));

    let special = Repo::new("supported-package-product-special-root-agent");
    let context = special.context();
    fs::remove_file(special.root.join(canonical)).expect("remove root agent for special file");
    let special_path = special.root.join(canonical);
    let encoded = std::ffi::CString::new(special_path.as_os_str().as_bytes())
        .expect("special root agent path");
    assert_eq!(unsafe { libc::mkfifo(encoded.as_ptr(), 0o600) }, 0);
    assert!(matches!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("special root agent accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable | ProductionPackageErrorId::ContextUnavailable
    ));
}

#[test]
fn root_agent_paths_cannot_be_laundered_through_resources_or_mismatched_names() {
    let laundered = Repo::new("supported-package-product-laundered-root-agents");
    let draft_path = laundered.root.join("plugin-manifest-draft.json");
    let mut draft: serde_json::Value =
        serde_json::from_slice(&fs::read(&draft_path).expect("read draft manifest"))
            .expect("parse draft manifest");
    draft["agents"] = json!([]);
    let resources = draft["resources"].as_array_mut().expect("draft resources");
    resources.extend(
        crate::agent_roles::CANONICAL_AGENT_ROLES
            .iter()
            .map(|role| json!(role.manifest_path)),
    );
    fs::write(
        &draft_path,
        serde_json::to_vec(&draft).expect("encode laundered manifest"),
    )
    .expect("write laundered manifest");
    let context = laundered.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("resource-laundered root agents accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let mismatched = Repo::new("supported-package-product-mismatched-root-agent-name");
    let draft_path = mismatched.root.join("plugin-manifest-draft.json");
    let mut draft: serde_json::Value =
        serde_json::from_slice(&fs::read(&draft_path).expect("read draft manifest"))
            .expect("parse draft manifest");
    draft["agents"][0]["name"] = json!(crate::agent_roles::CANONICAL_AGENT_ROLES[1].name);
    draft["agents"][1]["name"] = json!(crate::agent_roles::CANONICAL_AGENT_ROLES[0].name);
    fs::write(
        &draft_path,
        serde_json::to_vec(&draft).expect("encode mismatched manifest"),
    )
    .expect("write mismatched manifest");
    let context = mismatched.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("mismatched root agent names accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );
}

#[test]
fn manifest_absence_version_drift_and_mutate_restore_fail_closed() {
    let repo = Repo::new("supported-package-product-version-drift");
    write_draft(&repo.root, "0.0.18");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let error =
        capture_product_package(&context, &authority_catalog).expect_err("version drift accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::ManifestMismatch);

    write_draft(&repo.root, SUPPORTED_VERSION);
    write_supported_manifest(&repo.root, "0.0.18");
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
