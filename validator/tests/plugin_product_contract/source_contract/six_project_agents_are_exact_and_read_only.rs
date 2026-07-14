#[test]
fn six_project_agents_are_exact_and_read_only() {
    let directory = root().join(".codex/agents");
    let found = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("agent directory unavailable: {error}"))
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .path()
                .file_stem()
                .and_then(|value| value.to_str())
                .map(str::to_owned)
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        found,
        AGENTS.iter().map(|agent| agent.name.to_owned()).collect()
    );
    for agent in AGENTS {
        let source = read(&agent.path());
        assert!(source.contains(&format!("name = \"{}\"", agent.name)));
        assert!(source.contains("sandbox_mode = \"read-only\""));
    }
    let entries = live_descriptors();
    validate_descriptor_bundle(
        &entries,
        read("fixtures/plugin-product/root-wiring-request.json").as_bytes(),
        read("plugin-manifest-draft.json").as_bytes(),
        read(".codex-plugin/plugin.json").as_bytes(),
    )
    .unwrap_or_else(|error| panic!("semantic descriptor bundle invalid: {error:?}"));
}

#[test]
fn descriptor_unknown_missing_duplicate_semantic_authority_and_stale_controls_fail() {
    let live = live_descriptors();

    let mut unknown = live.clone();
    unknown.push((
        ".codex/agents/unknown-reviewer.toml".to_owned(),
        live[0].1.clone(),
    ));
    assert_eq!(
        validate_descriptor_set(&unknown),
        Err(DescriptorError::UnknownPath)
    );

    let mut missing = live.clone();
    missing.pop();
    assert_eq!(
        validate_descriptor_set(&missing),
        Err(DescriptorError::MissingDescriptor)
    );

    let mut duplicate = live.clone();
    let security = duplicate
        .iter_mut()
        .find(|(path, _)| path.ends_with("security-reviewer.toml"))
        .unwrap_or_else(|| panic!("security descriptor missing"));
    security.1 = String::from_utf8(security.1.clone())
        .unwrap()
        .replace("name = \"security-reviewer\"", "name = \"claim-falsifier\"")
        .into_bytes();
    assert_eq!(
        validate_descriptor_set(&duplicate),
        Err(DescriptorError::DuplicateAgent)
    );

    let mut semantic = live.clone();
    semantic[0].1 = String::from_utf8(semantic[0].1.clone())
        .unwrap()
        .replace(
            AGENTS[0].description,
            "Read-only but semantically substituted description.",
        )
        .into_bytes();
    assert_eq!(
        validate_descriptor_set(&semantic),
        Err(DescriptorError::SemanticMismatch)
    );

    let mut escalated = live.clone();
    escalated[0].1 = String::from_utf8(escalated[0].1.clone())
        .unwrap()
        .replace(
            "sandbox_mode = \"read-only\"",
            "sandbox_mode = \"workspace-write\"",
        )
        .into_bytes();
    assert_eq!(
        validate_descriptor_set(&escalated),
        Err(DescriptorError::AuthorityEscalation)
    );

    let mut request: serde_json::Value =
        serde_json::from_str(&read("fixtures/plugin-product/root-wiring-request.json")).unwrap();
    request["descriptor_bindings"][0]["sha256"] =
        serde_json::Value::String(format!("sha256:{}", "0".repeat(64)));
    assert_eq!(
        validate_descriptor_bindings(&live, &serde_json::to_vec(&request).unwrap()),
        Err(DescriptorError::StaleBinding)
    );
}

#[test]
fn package_membership_missing_duplicate_and_unknown_rows_fail_closed() {
    let plugin = read(".codex-plugin/plugin.json");
    let package = read("plugin-manifest-draft.json");
    validate_manifest_membership(package.as_bytes(), plugin.as_bytes()).unwrap();

    let mut missing: serde_json::Value = serde_json::from_str(&package).unwrap();
    missing["agents"].as_array_mut().unwrap().pop();
    assert_eq!(
        validate_manifest_membership(&serde_json::to_vec(&missing).unwrap(), plugin.as_bytes()),
        Err(DescriptorError::PackageMembership)
    );

    let mut duplicate: serde_json::Value = serde_json::from_str(&package).unwrap();
    let row = duplicate["agents"][0].clone();
    duplicate["agents"].as_array_mut().unwrap().push(row);
    assert_eq!(
        validate_manifest_membership(&serde_json::to_vec(&duplicate).unwrap(), plugin.as_bytes()),
        Err(DescriptorError::PackageMembership)
    );

    let mut unknown: serde_json::Value = serde_json::from_str(&package).unwrap();
    unknown["agents"][0]["path"] =
        serde_json::Value::String(".codex/agents/unknown.toml".to_owned());
    assert_eq!(
        validate_manifest_membership(&serde_json::to_vec(&unknown).unwrap(), plugin.as_bytes()),
        Err(DescriptorError::PackageMembership)
    );
}

#[test]
fn canonical_commands_match_the_typed_successor_catalog() {
    let catalog = read("validator/src/cli/successor/catalog.rs");
    for group in [
        "InspectTarget::Capabilities",
        "FitAction::Inspect",
        "FitAction::Plan",
        "FitAction::Apply",
        "FitAction::Verify",
        "CheckProfile::Routine",
        "CheckProfile::Strict",
        "SuccessorCommand::Diagnose",
        "SuccessorCommand::Prove",
        "ObserveAction::Query",
        "ObserveAction::Export",
        "EvalAction::Audit",
        "MigrateAction::Plan",
        "MigrateAction::Verify",
        "MigrateAction::Retire",
    ] {
        assert!(catalog.contains(group), "missing typed command {group}");
    }
}

#[test]
fn source_install_discovery_runtime_and_journey_ceilings_stay_separate() {
    let install = read("docs/install-and-visibility.md");
    for layer in [
        "**Source:**",
        "**Package:**",
        "**Marketplace:**",
        "**Install:**",
        "**Cache:**",
        "**App registry and Plugins UI:**",
        "**Discovery:**",
        "**Runtime:**",
        "**Product journey:**",
    ] {
        assert!(install.contains(layer), "missing proof layer {layer}");
    }
    assert!(install.contains("A successful lower layer does not raise a higher claim."));
}
