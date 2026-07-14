#[test]
fn two_concurrent_publishers_yield_one_exact_winner() {
    let repo = Repo::new("supported-package-product-writer-race");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    let output_root = Arc::new(OutputRoot::new("supported-package-product-writer-race"));
    let first_read = Arc::new(Barrier::new(2));

    let handles = (0..2)
        .map(|_| {
            let context = context.clone();
            let authority_catalog = authority_catalog.clone();
            let artifact = artifact.clone();
            let output_root = Arc::clone(&output_root);
            let first_read = Arc::clone(&first_read);
            std::thread::spawn(move || {
                let mut output = output_root.tree();
                first_read.wait();
                artifact.publish(
                    &context,
                    &authority_catalog,
                    &output_root.journey(&artifact),
                    &ExpectedTree::Absent,
                    &mut output,
                )
            })
        })
        .collect::<Vec<_>>();
    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().expect("publisher thread"))
        .collect::<Vec<_>>();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_err()).count(),
        1
    );
    assert_eq!(
        output_root
            .tree()
            .inspect(2, 65 * 1024 * 1024)
            .unwrap()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn partial_pair_and_failed_output_reconciliation_never_publish() {
    let repo = Repo::new("supported-package-product-output-false-pass");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    let partial = vec![TreeObject::regular(
        format!("{PLUGIN_ID}-{SUPPORTED_VERSION}.hugpkg"),
        0o644,
        artifact.snapshot().archive().to_vec(),
    )];
    let partial_sha256 = tree_sha256(&partial).expect("partial digest");
    let output_root = OutputRoot::new("supported-package-product-output-false-pass");
    let partial_dir = output_root
        .root
        .join("repository/packages/harness-ultragoal");
    fs::create_dir_all(&partial_dir).unwrap();
    fs::write(
        partial_dir.join(format!("{PLUGIN_ID}-{SUPPORTED_VERSION}.hugpkg")),
        artifact.snapshot().archive(),
    )
    .unwrap();
    let mut partial_output = output_root.tree();
    let error = artifact
        .publish(
            &context,
            &authority_catalog,
            &output_root.journey(&artifact),
            &ExpectedTree::ExactDigest(partial_sha256),
            &mut partial_output,
        )
        .expect_err("partial pair was completed");
    assert_eq!(error.id(), ProductionPackageErrorId::OutputFailed);
    assert_eq!(
        partial_output.inspect(2, 65 * 1024 * 1024).unwrap(),
        Some(partial)
    );

    fs::remove_dir_all(&partial_dir).unwrap();
    fs::create_dir_all(&partial_dir).unwrap();
    let inventory_only = vec![TreeObject::regular(
        format!("{PLUGIN_ID}-{SUPPORTED_VERSION}.inventory.json"),
        0o644,
        artifact.snapshot().inventory().to_vec(),
    )];
    let inventory_only_sha256 = tree_sha256(&inventory_only).expect("inventory-only digest");
    fs::write(
        partial_dir.join(format!("{PLUGIN_ID}-{SUPPORTED_VERSION}.inventory.json")),
        artifact.snapshot().inventory(),
    )
    .unwrap();
    let mut inventory_only_output = output_root.tree();
    let error = artifact
        .publish(
            &context,
            &authority_catalog,
            &output_root.journey(&artifact),
            &ExpectedTree::ExactDigest(inventory_only_sha256),
            &mut inventory_only_output,
        )
        .expect_err("inventory-only output was completed");
    assert_eq!(error.id(), ProductionPackageErrorId::OutputFailed);
    assert_eq!(
        inventory_only_output.inspect(2, 65 * 1024 * 1024).unwrap(),
        Some(inventory_only)
    );
}

#[test]
fn same_version_different_bytes_cannot_replace_published_pair() {
    let repo = Repo::new("supported-package-product-same-version-reuse");
    let first_context = repo.context();
    let first_catalog = catalog(&first_context);
    let first = capture_product_package(&first_context, &first_catalog).expect("first package");
    let output_root = OutputRoot::new("supported-package-product-same-version");
    let mut output = output_root.tree();
    let transaction = first
        .publish(
            &first_context,
            &first_catalog,
            &output_root.journey(&first),
            &ExpectedTree::Absent,
            &mut output,
        )
        .expect("first publish");
    let original_rows = output.inspect(2, 65 * 1024 * 1024).unwrap().unwrap();

    fs::write(repo.root.join("skills/prove/SKILL.md"), "different bytes\n")
        .expect("mutate product source");
    let second_context = repo.context();
    let second_catalog = catalog(&second_context);
    let second = capture_product_package(&second_context, &second_catalog).expect("second package");
    assert_ne!(first.snapshot().archive(), second.snapshot().archive());
    let error = second
        .publish(
            &second_context,
            &second_catalog,
            &output_root.journey(&second),
            &ExpectedTree::ExactDigest(transaction.output_tree_sha256().to_owned()),
            &mut output,
        )
        .expect_err("same-version different bytes replaced output");
    assert_eq!(error.id(), ProductionPackageErrorId::OutputFailed);
    assert_eq!(
        output.inspect(2, 65 * 1024 * 1024).unwrap().unwrap(),
        original_rows
    );
}
