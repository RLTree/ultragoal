#[test]
fn every_artifact_binding_dimension_is_checked_before_output_effects() {
    let repo = Repo::new("supported-package-product-binding-substitution");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");

    let mut substitutions = Vec::new();
    let mut candidate = artifact.clone();
    candidate.candidate_id = format!("sha256:{}", "1".repeat(64));
    substitutions.push(candidate);
    let mut artifact_catalog = artifact.clone();
    artifact_catalog.catalog_id = format!("sha256:{}", "2".repeat(64));
    substitutions.push(artifact_catalog);
    let mut source_inventory = artifact.clone();
    source_inventory.source_inventory.push(b' ');
    substitutions.push(source_inventory);
    let mut source_snapshot = artifact.clone();
    source_snapshot.source_snapshot_id = format!("sha256:{}", "3".repeat(64));
    substitutions.push(source_snapshot);
    let mut plan_context = artifact.clone();
    plan_context.plan.context_id = format!("sha256:{}", "4".repeat(64));
    substitutions.push(plan_context);
    let mut plan_version = artifact.clone();
    plan_version.plan.version = "0.0.16".to_string();
    substitutions.push(plan_version);
    let mut plan_catalog = artifact.clone();
    plan_catalog.plan.catalog_id = format!("sha256:{}", "5".repeat(64));
    substitutions.push(plan_catalog);
    let mut plan_inventory = artifact.clone();
    plan_inventory.plan.accepted_inventory_sha256 = format!("sha256:{}", "6".repeat(64));
    substitutions.push(plan_inventory);
    let mut plan_tree = artifact.clone();
    plan_tree.plan.source_tree_sha256 = format!("sha256:{}", "7".repeat(64));
    substitutions.push(plan_tree);
    let mut plan_mode = artifact.clone();
    plan_mode.plan.entries[0].mode = 0o755;
    substitutions.push(plan_mode);
    let mut plan_row = artifact.clone();
    plan_row.plan.entries[0].bytes.push(b' ');
    substitutions.push(plan_row);
    let mut binding = artifact.clone();
    binding.binding = binding
        .binding
        .substituted("inventory_sha256", &format!("sha256:{}", "8".repeat(64)));
    substitutions.push(binding);

    for altered in substitutions {
        let output_root = OutputRoot::new("supported-package-product-binding-substitution");
        let mut output = output_root.tree();
        assert!(
            altered
                .publish(
                    &context,
                    &authority_catalog,
                    &output_root.journey(&altered),
                    &ExpectedTree::Absent,
                    &mut output,
                )
                .is_err()
        );
        assert!(output.inspect(2, 65 * 1024 * 1024).unwrap().is_none());
    }
}

#[test]
fn case_collision_symlink_special_file_and_unsafe_mode_fail_closed() {
    let case_collision = Repo::new("supported-package-product-case-collision");
    let alias_root = case_collision.root.join("skills/PROVE");
    match fs::create_dir(&alias_root) {
        Ok(()) => {
            fs::write(alias_root.join("SKILL.md"), "case alias\n").expect("case alias file");
            let context = case_collision.context();
            assert_eq!(
                capture_product_package(&context, &catalog(&context))
                    .expect_err("case collision accepted")
                    .id(),
                ProductionPackageErrorId::SourceUnavailable
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            // The host filesystem cannot represent a distinct ASCII case-colliding entry.
            // The synthetic capture control covers the production classifier on this host.
        }
        Err(error) => panic!("case alias directory: {error}"),
    }

    let linked = Repo::new("supported-package-product-symlink");
    let agent = linked.root.join("skills/prove/agents/openai.yaml");
    fs::remove_file(&agent).expect("remove canonical agent");
    symlink(linked.root.join("skills/prove/SKILL.md"), &agent).expect("symlink agent");
    let context = linked.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("symlink accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let special = Repo::new("supported-package-product-special");
    let agent = special.root.join("skills/prove/agents/openai.yaml");
    fs::remove_file(&agent).expect("remove canonical agent");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let socket_path =
        PathBuf::from("/tmp").join(format!("hul-pkg-{}-{nonce}.sock", std::process::id()));
    let _listener = UnixListener::bind(&socket_path).expect("short special socket");
    fs::rename(&socket_path, &agent).expect("move special socket into canonical member");
    let context = special.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("special file accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let executable = Repo::new("supported-package-product-unsafe-mode");
    let skill = executable.root.join("skills/prove/SKILL.md");
    let mut permissions = fs::metadata(&skill).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&skill, permissions).expect("set executable mode");
    let context = executable.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("unsafe mode accepted")
            .id(),
        ProductionPackageErrorId::MembershipMismatch
    );
}

#[test]
fn duplicate_json_path_escape_and_oversized_source_fail_closed() {
    let duplicate = Repo::new("supported-package-product-duplicate-json");
    fs::write(
        duplicate.root.join(SUPPORTED_MANIFEST_PATH),
        br#"{"name":"harness-ultragoal","name":"harness-ultragoal"}"#,
    )
    .expect("duplicate-key manifest");
    let context = duplicate.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("duplicate JSON accepted")
            .id(),
        ProductionPackageErrorId::ManifestMismatch
    );

    let escaped = Repo::new("supported-package-product-path-escape");
    let draft_path = escaped.root.join("plugin-manifest-draft.json");
    let mut draft: serde_json::Value =
        serde_json::from_slice(&fs::read(&draft_path).unwrap()).unwrap();
    draft["skills"][0]["path"] = json!("../canary/SKILL.md");
    fs::write(&draft_path, serde_json::to_vec(&draft).unwrap()).expect("escaped draft");
    let context = escaped.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("path escape accepted")
            .id(),
        ProductionPackageErrorId::SourceUnavailable
    );

    let oversized = Repo::new("supported-package-product-oversized-entry");
    fs::OpenOptions::new()
        .write(true)
        .open(oversized.root.join("skills/prove/SKILL.md"))
        .unwrap()
        .set_len(4 * 1024 * 1024 + 1)
        .unwrap();
    let context = oversized.context();
    assert_eq!(
        capture_product_package(&context, &catalog(&context))
            .expect_err("oversized package entry accepted")
            .id(),
        ProductionPackageErrorId::ArchiveMismatch
    );
}
