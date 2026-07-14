fn assert_catalog_blocks_route(repo: &TestRepo) {
    let catalog = catalog(repo);
    let legacy = entry(&catalog, CASES[0]);
    assert_eq!(legacy.authority_state, AuthorityState::Legacy);
    assert_eq!(legacy.active_status, ActiveStatus::Active);
    assert!(catalog.findings().iter().any(|finding| {
        finding.code == "invalid_agent_route_transition"
            && finding.entry_id.as_deref() == Some(legacy.stable_id.as_str())
    }));
    assert!(catalog.findings().iter().any(|finding| {
        finding.code == "parallel_authority"
            && finding.entry_id.as_deref() == Some(legacy.stable_id.as_str())
    }));
}

fn assert_reader_drift_blocks_route(repo: &TestRepo) {
    repo.commit();
    assert_catalog_blocks_route(repo);
}

fn assert_non_reader_allows_route(repo: &TestRepo) {
    repo.commit();
    let catalog = catalog(repo);
    let legacy = entry(&catalog, CASES[0]);
    assert_eq!(legacy.authority_state, AuthorityState::Context);
    assert_eq!(legacy.active_status, ActiveStatus::ContextOnly);
    assert!(!catalog.findings().iter().any(|finding| {
        finding.entry_id.as_deref() == Some(legacy.stable_id.as_str())
            && matches!(
                finding.code.as_str(),
                "invalid_agent_route_transition" | "parallel_authority"
            )
    }));
}

fn manifest(repo: &TestRepo) -> Value {
    serde_json::from_slice(&fs::read(repo.root.join("plugin-manifest-draft.json")).unwrap())
        .unwrap()
}

fn write_manifest(repo: &TestRepo, value: &Value) {
    repo.write(
        "plugin-manifest-draft.json",
        &serde_json::to_vec(value).unwrap(),
    );
}

#[test]
fn manifest_and_resource_legacy_paths_cannot_reuse_phase_a_receipt() {
    for mutation in ["agent-row", "resource-row"] {
        let repo = TestRepo::new(&format!("agent-route-{mutation}"));
        prepare(&repo, &[CASES[0]], true);
        let receipt = fs::read(repo.root.join(READER_PROOF)).unwrap();
        let mut value = manifest(&repo);
        match mutation {
            "agent-row" => {
                value["agents"][0]["path"] = json!("agents/claim-falsifier.toml");
            }
            _ => value["resources"]
                .as_array_mut()
                .unwrap()
                .push(json!("custom-agents/smuggled.toml")),
        }
        write_manifest(&repo, &value);
        assert_eq!(fs::read(repo.root.join(READER_PROOF)).unwrap(), receipt);
        assert_reader_drift_blocks_route(&repo);
    }
}

#[test]
fn current_agent_discovery_reader_omission_and_mutation_block_routes() {
    for path in super::CURRENT_AGENT_DISCOVERY_READERS {
        let label = path.rsplit('/').next().unwrap().trim_end_matches(".rs");

        let omitted = TestRepo::new(&format!("agent-route-{label}-reader-omitted"));
        prepare(&omitted, &[CASES[0]], true);
        fs::remove_file(omitted.root.join(path)).unwrap();
        assert_reader_drift_blocks_route(&omitted);

        let mutated = TestRepo::new(&format!("agent-route-{label}-reader-mutated"));
        prepare(&mutated, &[CASES[0]], true);
        let mut bytes = fs::read(mutated.root.join(path)).unwrap();
        bytes.extend_from_slice(b"\n// exact positive reader drift\n");
        mutated.write(path, &bytes);
        assert_reader_drift_blocks_route(&mutated);
    }
}

#[test]
fn compatibility_and_non_rust_positive_readers_are_not_implicit_allowlist() {
    for mutation in [
        "compatibility-rust",
        "harness-javascript",
        "extensionless-script",
    ] {
        let repo = TestRepo::new(&format!("agent-route-{mutation}"));
        prepare(&repo, &[CASES[0]], true);
        match mutation {
            "compatibility-rust" => repo.write(
                "validator/src/inventory/compatibility/unbound_reader.rs",
                br#"fn read() { let _ = std::fs::read(".codex/".to_owned() + "agents/repo-recon.toml"); }"#,
            ),
            "harness-javascript" => repo.write(
                ".harness/unbound-agent-reader.js",
                br#"const path = "custom-" + "agents/reintroduced.toml"; readFileSync(path);"#,
            ),
            _ => repo.write(
                "scripts/unbound-agent-reader",
                br#"read(".codex/" + "agents/repo-recon.toml")"#,
            ),
        }
        assert_reader_drift_blocks_route(&repo);
    }
}
