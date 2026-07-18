#[test]
fn absent_or_stale_witness_sources_fail_closed_without_echoing_bytes() {
    let absent = TestRepo::new("activation-source-absent");
    establish_fixture_authority(&absent);
    let context = LiveContext::build(inventory_request(&absent.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    assert!(
        catalog
            .findings()
            .iter()
            .any(|finding| { finding.code == "activation_witness_source_set_unverified" })
    );
    assert!(
        catalog
            .entries()
            .iter()
            .filter(|entry| entry.stable_id.starts_with("API:"))
            .all(|entry| entry.active_status != ActiveStatus::Active)
    );

    for (label, relative) in [
        (
            "activation-source-stale-semantic",
            "validator/src/inventory/registry/semantic.rs",
        ),
        (
            "activation-source-stale-status",
            "validator/src/inventory/types/mod.rs",
        ),
        (
            "activation-source-stale-audit",
            "validator/src/inventory/validate/mod.rs",
        ),
        (
            "activation-source-stale-export",
            "validator/examples/hct_inventory.rs",
        ),
        (
            "activation-source-stale-external-witness",
            "validator/tests/public_api_witness.rs",
        ),
    ] {
        let stale = TestRepo::new(label);
        copy_sources(&stale);
        establish_fixture_authority(&stale);
        stale.write(relative, b"SECRET_CANARY_STALE\n");
        stale.commit();
        let context = LiveContext::build(inventory_request(&stale.root)).unwrap();
        let error = InventoryBuilder::new(&context).build().unwrap_err();
        assert!(error.to_string().ends_with("stale-input"), "{relative}");
        assert!(!error.to_string().contains("SECRET_CANARY_STALE"));
    }

    let private_export = TestRepo::new("activation-source-private-root-export");
    copy_sources(&private_export);
    establish_fixture_authority(&private_export);
    let api_before = fs::read(private_export.root.join("validator/src/api_witness.rs")).unwrap();
    let lib_path = private_export.root.join("validator/src/lib.rs");
    let lib = fs::read_to_string(&lib_path).unwrap();
    let private = lib.replace("pub mod inventory;", "mod inventory;");
    assert_ne!(private, lib, "root export canary did not match lib.rs");
    fs::write(&lib_path, private).unwrap();
    assert_eq!(
        fs::read(private_export.root.join("validator/src/api_witness.rs")).unwrap(),
        api_before,
        "internal API witness changed with the root-export control"
    );
    private_export.commit();
    let context = LiveContext::build(inventory_request(&private_export.root)).unwrap();
    assert!(
        InventoryBuilder::new(&context)
            .build()
            .unwrap_err()
            .to_string()
            .ends_with("stale-input")
    );

    for (label, relative) in [
        (
            "activation-source-omitted-semantic",
            "validator/src/inventory/registry/semantic.rs",
        ),
        ("activation-source-omitted-lib", "validator/src/lib.rs"),
        (
            "activation-source-omitted-external-witness",
            "validator/tests/public_api_witness.rs",
        ),
    ] {
        let omitted = TestRepo::new(label);
        copy_sources(&omitted);
        establish_fixture_authority(&omitted);
        fs::remove_file(omitted.root.join(relative)).unwrap();
        omitted.commit();
        let context = LiveContext::build(inventory_request(&omitted.root)).unwrap();
        assert!(
            InventoryBuilder::new(&context)
                .build()
                .unwrap_err()
                .to_string()
                .ends_with("missing-row"),
            "{relative}"
        );
    }
}

#[cfg(unix)]
#[test]
fn witness_source_symlink_hardlink_and_fifo_are_rejected_before_activation() {
    use std::os::unix::fs::symlink;

    fn assert_special_controls(relative: &str, link_target: &str, label: &str) {
        let linked = TestRepo::new(&format!("activation-{label}-symlink"));
        copy_sources(&linked);
        establish_fixture_authority(&linked);
        fs::remove_file(linked.root.join(relative)).unwrap();
        symlink(link_target, linked.root.join(relative)).unwrap();
        linked.commit();
        let context = LiveContext::build(inventory_request(&linked.root)).unwrap();
        assert!(
            InventoryBuilder::new(&context)
                .build()
                .unwrap_err()
                .to_string()
                .ends_with("unsafe-input")
        );

        let hard = TestRepo::new(&format!("activation-{label}-hardlink"));
        copy_sources(&hard);
        establish_fixture_authority(&hard);
        let source = hard.root.join(relative);
        let target = hard
            .root
            .join(format!("validator/{label}-hardlink-target.rs"));
        fs::remove_file(&source).unwrap();
        fs::write(&target, fs::read(live_root().join(relative)).unwrap()).unwrap();
        fs::hard_link(&target, &source).unwrap();
        hard.commit();
        let context = LiveContext::build(inventory_request(&hard.root)).unwrap();
        assert!(
            InventoryBuilder::new(&context)
                .build()
                .unwrap_err()
                .to_string()
                .ends_with("unsafe-input")
        );

        let fifo = TestRepo::new(&format!("activation-{label}-fifo"));
        copy_sources(&fifo);
        establish_fixture_authority(&fifo);
        fs::remove_file(fifo.root.join(relative)).unwrap();
        fifo.commit();
        assert!(
            std::process::Command::new("mkfifo")
                .arg(fifo.root.join(relative))
                .status()
                .unwrap()
                .success()
        );
        let context = LiveContext::build(inventory_request(&fifo.root)).unwrap();
        assert!(
            InventoryBuilder::new(&context)
                .build()
                .unwrap_err()
                .to_string()
                .ends_with("unsafe-input")
        );
    }

    for (relative, target, label) in [
        (
            "validator/src/inventory/registry/semantic.rs",
            "topology.rs",
            "semantic",
        ),
        ("validator/src/lib.rs", "api_witness.rs", "lib"),
        (
            "validator/tests/public_api_witness.rs",
            "../src/api_witness.rs",
            "public-witness",
        ),
    ] {
        assert_special_controls(relative, target, label);
    }
}
