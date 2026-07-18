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
    "validator/src/inventory/registry/frontier/mod.rs",
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

#[test]
fn ready_frontier_rejects_unintegrated_dependencies_and_unexpected_lanes() {
    let blocked_dependency = source_repo("ready-blocked-dependency");
    set_lane_states(&blocked_dependency, &[("N03", "blocked")], None);
    assert_inventory_error(
        &blocked_dependency,
        "scheduler ready frontier is not dependency closed",
    );

    let early_downstream = source_repo("ready-early-downstream");
    set_lane_states(
        &early_downstream,
        &[
            ("N04", "blocked"),
            ("N05", "blocked"),
            ("N06", "blocked"),
            ("N07", "blocked"),
            ("N08", "ready"),
        ],
        Some(&["N08"]),
    );
    assert_inventory_error(
        &early_downstream,
        "scheduler frontier has unexpected ready lanes",
    );
}

#[test]
fn scope_authority_rejects_effect_overlap_and_root_only_paths() {
    let overlapping = source_repo("scope-effect-overlap");
    mutate_scope(&overlapping, "WS-FIT", |scope| {
        scope["effects"] = serde_json::json!(["package-plan"]);
    });
    assert_inventory_error(&overlapping, "scope effect authority overlaps");

    let root_only = source_repo("scope-root-only-path");
    mutate_scope(&root_only, "WS-FIT", |scope| {
        scope["owned_roots"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::Value::String("Cargo.toml".to_owned()));
    });
    assert_inventory_error(&root_only, "scope owns a root-only surface");
}

fn set_lane_states(repo: &TestRepo, states: &[(&str, &str)], eligible: Option<&[&str]>) {
    let path = repo.root.join("LANE_REGISTRY.json");
    let mut registry: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for (lane_id, state) in states {
        let lane = registry["lanes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|lane| lane["id"] == *lane_id)
            .unwrap();
        lane["state"] = serde_json::Value::String((*state).to_owned());
    }
    if let Some(eligible) = eligible {
        registry["pre_adoption_source"]["eligible_scheduler_nodes"] = serde_json::Value::Array(
            eligible
                .iter()
                .map(|lane| serde_json::Value::String((*lane).to_owned()))
                .collect(),
        );
    }
    fs::write(path, serde_json::to_vec(&registry).unwrap()).unwrap();
}

fn mutate_scope(repo: &TestRepo, scope_id: &str, mutate: impl FnOnce(&mut serde_json::Value)) {
    let path = repo.root.join("LANE_REGISTRY.json");
    let mut registry: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let scope = registry["scope_mappings"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|scope| scope["scope_id"] == scope_id)
        .unwrap();
    mutate(scope);
    fs::write(path, serde_json::to_vec(&registry).unwrap()).unwrap();
}

fn assert_inventory_error(repo: &TestRepo, expected: &str) {
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let error = InventoryBuilder::new(&context).build().unwrap_err();
    assert_eq!(error.to_string(), format!("invalid registry: {expected}"));
}
