fn manifest(prompts: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "name": "harness-ultragoal",
        "version": "1.2.3-beta.1+codex.local-1",
        "description": "fixture plugin",
        "author": {"name": "Inventory Test"},
        "skills": "./skills/",
        "interface": {
            "displayName": "Harness Ultragoal",
            "shortDescription": "fixture",
            "longDescription": "fixture plugin manifest",
            "developerName": "Inventory Test",
            "category": "Productivity",
            "capabilities": ["Read"],
            "defaultPrompt": prompts
        }
    })
}

fn catalog(repo: &TestRepo) -> crate::inventory::AuthorityCatalog {
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    InventoryBuilder::new(&context).build().unwrap()
}

fn has_code(catalog: &crate::inventory::AuthorityCatalog, code: &str) -> bool {
    catalog
        .findings()
        .iter()
        .any(|finding| finding.code == code)
}

#[test]
fn supported_manifest_shape_is_validated_without_legacy_draft_equivalence() {
    let repo = TestRepo::new("supported-plugin-manifest");
    repo.write(
        ".codex-plugin/plugin.json",
        &serde_json::to_vec(&manifest(serde_json::json!(["Inspect this repository."]))).unwrap(),
    );
    repo.write(
        "plugin-manifest-draft.json",
        br#"{"unsupported_legacy_shape":true}"#,
    );
    repo.commit();

    let catalog = catalog(&repo);
    for absent in [
        "invalid_source_plugin_manifest",
        "invalid_plugin_manifest_path",
        "invalid_plugin_default_prompt",
        "projection_drift",
    ] {
        assert!(!has_code(&catalog, absent));
    }
    assert!(catalog.findings().iter().any(|finding| {
        finding.code == "parallel_authority"
            && finding.relative_path.as_deref() == Some("plugin-manifest-draft.json")
    }));
}

#[test]
fn current_minimal_manifest_keeps_inline_hooks_as_unsupported_legacy_context() {
    let repo = TestRepo::new("minimal-plugin-manifest");
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"fixture-plugin","version":"1.0.0","description":"fixture","hooks":{"SessionStart":[{"matcher":"startup","hooks":[{"type":"command","command":"true","timeout":5,"statusMessage":"fixture"}]}]}}"#,
    );
    repo.commit();
    let catalog = catalog(&repo);
    for absent in [
        "invalid_source_plugin_manifest",
        "invalid_plugin_hooks",
        "invalid_plugin_default_prompt",
    ] {
        assert!(!has_code(&catalog, absent));
    }
    assert!(has_code(&catalog, "unsupported_plugin_manifest_field"));
    assert!(has_code(&catalog, "plugin_hook_trust_required"));
    assert!(
        !catalog
            .entries()
            .iter()
            .any(|entry| entry.stable_id.starts_with("PLUGIN-HOOKS:"))
    );
}

#[test]
fn canonical_hook_companion_is_validated_and_prefixless_or_malformed_hooks_fail() {
    let repo = TestRepo::new("plugin-hook-companion");
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"fixture-plugin","version":"1.0.0","description":"fixture","hooks":"./hooks/session.json"}"#,
    );
    repo.write(
        "hooks/session.json",
        br#"{"description":"fixture","hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"true"}]}]}}"#,
    );
    repo.commit();
    let valid = catalog(&repo);
    assert!(!has_code(&valid, "invalid_plugin_hooks"));
    assert!(has_code(&valid, "unsupported_plugin_manifest_field"));
    assert!(has_code(&valid, "plugin_hook_trust_required"));
    assert!(valid.entries().iter().any(|entry| {
        entry.stable_id == "PLUGIN-HOOKS:hooks/session.json"
            && entry.active_status == crate::inventory::ActiveStatus::Candidate
            && entry
                .input_provenance
                .contains(&"hook-source:manifest-explicit".to_owned())
    }));

    let repo = TestRepo::new("plugin-hook-prefix");
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"fixture-plugin","version":"1.0.0","description":"fixture","hooks":"hooks/session.json"}"#,
    );
    repo.write(
        "hooks/session.json",
        br#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"true"}]}]}}"#,
    );
    repo.commit();
    let prefixless = catalog(&repo);
    assert!(has_code(&prefixless, "unsupported_plugin_manifest_field"));
    assert!(has_code(&prefixless, "invalid_plugin_hooks"));

    let repo = TestRepo::new("plugin-hook-malformed-companion");
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"fixture-plugin","version":"1.0.0","description":"fixture","hooks":"./hooks/bad.json"}"#,
    );
    repo.write(
        "hooks/bad.json",
        br#"{"hooks":{"SessionStart":[],"SessionStart":[]}}"#,
    );
    repo.commit();
    let malformed = catalog(&repo);
    assert!(has_code(&malformed, "unsupported_plugin_manifest_field"));
    assert!(has_code(&malformed, "invalid_plugin_hooks"));
    assert!(malformed.entries().iter().any(|entry| {
        entry.stable_id == "PLUGIN-HOOKS:hooks/bad.json"
            && entry.active_status == crate::inventory::ActiveStatus::ContextOnly
    }));

    let repo = TestRepo::new("plugin-hook-malformed");
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"fixture-plugin","version":"1.0.0","description":"fixture","hooks":{"SessionStart":[{"type":"command","command":"true"}]}}"#,
    );
    repo.commit();
    let inline = catalog(&repo);
    assert!(has_code(&inline, "unsupported_plugin_manifest_field"));
    assert!(has_code(&inline, "invalid_plugin_hooks"));

    let repo = TestRepo::new("plugin-hook-malformed-regex");
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"fixture-plugin","version":"1.0.0","description":"fixture","hooks":{"SessionStart":[{"matcher":"[","hooks":[{"type":"command","command":"true"}]}]}}"#,
    );
    repo.commit();
    let regex = catalog(&repo);
    assert!(has_code(&regex, "unsupported_plugin_manifest_field"));
    assert!(has_code(&regex, "invalid_plugin_hooks"));
}
