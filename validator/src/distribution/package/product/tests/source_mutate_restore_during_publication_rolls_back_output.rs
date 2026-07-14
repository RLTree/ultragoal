#[test]
fn source_mutate_restore_during_publication_rolls_back_output() {
    let repo = Repo::new("supported-package-product-publication-mutate-restore");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    let path = repo.root.join("skills/prove/SKILL.md");
    let original = fs::read(&path).expect("original source");
    let hook_path = path.clone();
    let hook_original = original.clone();
    let mut output = MemoryOutput {
        before_first_transition: Some(Box::new(move || {
            fs::write(&hook_path, b"mutated during output\n").expect("mutate source");
            fs::write(&hook_path, hook_original).expect("restore source");
        })),
        ..MemoryOutput::default()
    };

    let error = artifact
        .publish(
            &context,
            &authority_catalog,
            &ExpectedTree::Absent,
            &mut output,
        )
        .expect_err("mutate-restore crossed publication");
    assert!(matches!(
        error.id(),
        ProductionPackageErrorId::SourceUnavailable | ProductionPackageErrorId::ContextUnavailable
    ));
    assert!(
        output.rows.is_none(),
        "failed publication was not rolled back"
    );
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn two_concurrent_publishers_yield_one_exact_winner() {
    let repo = Repo::new("supported-package-product-writer-race");
    let context = repo.context();
    let authority_catalog = catalog(&context);
    let artifact = capture_product_package(&context, &authority_catalog).expect("package");
    let rows = Arc::new(Mutex::new(None));
    let first_read = Arc::new(Barrier::new(2));

    let handles = (0..2)
        .map(|_| {
            let context = context.clone();
            let authority_catalog = authority_catalog.clone();
            let artifact = artifact.clone();
            let rows = Arc::clone(&rows);
            let first_read = Arc::clone(&first_read);
            std::thread::spawn(move || {
                let mut output = ConcurrentOutput {
                    rows,
                    first_read,
                    reads: 0,
                };
                artifact.publish(
                    &context,
                    &authority_catalog,
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
    assert_eq!(rows.lock().unwrap().as_ref().unwrap().len(), 2);
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
    let mut partial_output = MemoryOutput {
        rows: Some(partial.clone()),
        ..MemoryOutput::default()
    };
    let error = artifact
        .publish(
            &context,
            &authority_catalog,
            &ExpectedTree::ExactDigest(partial_sha256),
            &mut partial_output,
        )
        .expect_err("partial pair was completed");
    assert_eq!(error.id(), ProductionPackageErrorId::OutputFailed);
    assert_eq!(partial_output.rows, Some(partial));
    assert_eq!(partial_output.transitions, 0);

    let inventory_only = vec![TreeObject::regular(
        format!("{PLUGIN_ID}-{SUPPORTED_VERSION}.inventory.json"),
        0o644,
        artifact.snapshot().inventory().to_vec(),
    )];
    let inventory_only_sha256 = tree_sha256(&inventory_only).expect("inventory-only digest");
    let mut inventory_only_output = MemoryOutput {
        rows: Some(inventory_only.clone()),
        ..MemoryOutput::default()
    };
    let error = artifact
        .publish(
            &context,
            &authority_catalog,
            &ExpectedTree::ExactDigest(inventory_only_sha256),
            &mut inventory_only_output,
        )
        .expect_err("inventory-only output was completed");
    assert_eq!(error.id(), ProductionPackageErrorId::OutputFailed);
    assert_eq!(inventory_only_output.rows, Some(inventory_only));
    assert_eq!(inventory_only_output.transitions, 0);

    let mut corrupt_output = MemoryOutput {
        corrupt_on_read: Some(2),
        ..MemoryOutput::default()
    };
    let error = artifact
        .publish(
            &context,
            &authority_catalog,
            &ExpectedTree::Absent,
            &mut corrupt_output,
        )
        .expect_err("corrupt post-write pair was accepted");
    assert_eq!(error.id(), ProductionPackageErrorId::OutputFailed);
    assert!(corrupt_output.rows.is_none());
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
