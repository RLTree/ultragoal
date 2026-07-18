const SOURCES: &[&str] = &[
    "validator/src/api_witness.rs",
    "validator/src/lib.rs",
    "validator/tests/public_api_witness.rs",
    "validator/src/command_witness.rs",
    "validator/src/cli/successor/catalog/mod.rs",
    "validator/src/cli/successor/catalog/evaluation_and_migration.rs",
    "validator/src/cli/successor/catalog/repository_fit_and_checks.rs",
    "validator/src/cli/successor/catalog/inspection.rs",
    "validator/src/cli/successor/catalog/observability_and_package.rs",
    "validator/src/cli/successor/catalog/options.rs",
    "validator/src/cli/successor/command_contract/mod.rs",
    "validator/src/cli/successor/command_contract/arguments.rs",
    "validator/src/cli/successor/command_contract/commands.rs",
    "validator/src/cli/successor/command_contract/descriptor.rs",
    "validator/src/cli/successor/command_contract/exit.rs",
    "validator/src/cli/successor/command_contract/invocation.rs",
    "validator/src/inventory/mod.rs",
    "validator/src/inventory/builder.rs",
    "validator/src/inventory/digest.rs",
    "validator/src/inventory/fs.rs",
    "validator/src/inventory/projection.rs",
    "validator/src/inventory/registry/command_activation/mod.rs",
    "validator/src/inventory/registry/command_activation/guard.rs",
    "validator/src/inventory/registry/command_activation/exact_source_manifest.rs",
    "validator/src/inventory/registry/data.rs",
    "validator/src/inventory/registry/integrity.rs",
    "validator/src/inventory/registry/frontier.rs",
    "validator/src/inventory/registry/load.rs",
    "validator/src/inventory/registry/mod.rs",
    "validator/src/inventory/registry/semantic.rs",
    "validator/src/inventory/registry/sources.rs",
    "validator/src/inventory/registry/topology.rs",
    "validator/src/inventory/types/mod.rs",
    "validator/src/inventory/validate/duplicates.rs",
    "validator/src/inventory/validate/mod.rs",
    "validator/examples/hct_inventory.rs",
];

fn copy_sources(repo: &TestRepo) {
    let source_root = live_root();
    for relative in SOURCES {
        repo.write(relative, &fs::read(source_root.join(relative)).unwrap());
    }
}

fn source_repo(label: &str) -> TestRepo {
    let repo = TestRepo::new(label);
    repo.write(".gitignore", b"target/\n");
    copy_sources(&repo);
    repo.commit();
    repo
}

#[test]
fn exact_current_witness_sources_activate_apis_but_not_command_definitions() {
    let repo = source_repo("activation-current");
    let before = snapshot(&repo.root);
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    let after = snapshot(&repo.root);
    assert_eq!(
        before, after,
        "inventory read path wrote into the repository"
    );
    assert!(
        !catalog
            .findings()
            .iter()
            .any(|finding| finding.code == "activation_witness_source_set_unverified")
    );
    assert!(
        catalog
            .entries()
            .iter()
            .filter(|entry| entry.stable_id.starts_with("API:"))
            .any(|entry| entry.active_status == ActiveStatus::Active)
    );
    assert!(
        catalog
            .entries()
            .iter()
            .filter(|entry| entry.stable_id.starts_with("COMMAND:"))
            .all(|entry| entry.active_status == ActiveStatus::Candidate)
    );
    assert_eq!(
        catalog
            .source_registry_counts()
            .get("verified_command_handler_activations"),
        Some(&0)
    );
    assert_eq!(
        catalog
            .source_registry_counts()
            .get("activation_witness_sources"),
        Some(&SOURCES.len())
    );
    for entry in catalog
        .entries()
        .iter()
        .filter(|entry| entry.generator.as_deref() == Some("HCT-INVENTORY:compiled-api-witness"))
    {
        for source in SOURCES {
            assert!(
                entry.input_provenance.iter().any(|path| path == source),
                "{} omits activation dependency {source}",
                entry.stable_id
            );
        }
    }
    let closure = catalog.closure_status();
    assert!(!closure.is_closed());
    assert!(
        closure
            .open_obligations_by_code()
            .contains_key("candidate_component_not_active")
    );
}
