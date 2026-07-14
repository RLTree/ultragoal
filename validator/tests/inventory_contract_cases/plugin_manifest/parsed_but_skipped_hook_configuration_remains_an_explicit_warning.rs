#[test]
fn parsed_but_skipped_hook_configuration_remains_an_explicit_warning() {
    for (name, hooks) in [
        (
            "plugin-hook-inactive-handler",
            serde_json::json!({"SessionStart":[{"hooks":[{"type":"prompt","prompt":"fixture"}]}]}),
        ),
        (
            "plugin-hook-ignored-matcher",
            serde_json::json!({"Stop":[{"matcher":"Bash","hooks":[{"type":"command","command":"true"}]}]}),
        ),
    ] {
        let repo = TestRepo::new(name);
        let mut value = serde_json::json!({
            "name":"fixture-plugin",
            "version":"1.0.0",
            "description":"fixture"
        });
        value["hooks"] = hooks;
        repo.write(
            ".codex-plugin/plugin.json",
            &serde_json::to_vec(&value).unwrap(),
        );
        repo.commit();
        let catalog = catalog(&repo);
        assert!(has_code(&catalog, "unsupported_plugin_manifest_field"));
        assert!(!has_code(&catalog, "invalid_plugin_hooks"));
        let finding = catalog
            .findings()
            .iter()
            .find(|finding| finding.code == "inactive_plugin_hook_configuration")
            .unwrap();
        assert_eq!(finding.severity, crate::inventory::FindingSeverity::Warning);
    }
}

#[test]
fn product_manifest_requires_its_canonical_skill_root() {
    let repo = TestRepo::new("product-plugin-missing-skills");
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"harness-ultragoal","version":"1.0.0","description":"fixture"}"#,
    );
    repo.commit();
    assert!(has_code(&catalog(&repo), "invalid_plugin_manifest_path"));
}

#[test]
fn unknown_fields_duplicates_and_placeholders_fail_without_echoing_canaries() {
    let repo = TestRepo::new("invalid-plugin-manifest");
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"harness-ultragoal","name":"duplicate","version":"1.0.0","description":"[TODO: SECRET_CANARY]","author":{"name":"x"},"hooks":"SECRET_CANARY","interface":{"displayName":"x","shortDescription":"x","longDescription":"x","developerName":"x","category":"x","capabilities":["Read"],"defaultPrompt":["x"]}}"#,
    );
    repo.commit();

    let catalog = catalog(&repo);
    assert!(has_code(&catalog, "invalid_source_plugin_manifest"));
    assert!(has_code(&catalog, "plugin_manifest_placeholder"));
    let bytes = catalog.to_canonical_json().unwrap();
    assert!(!String::from_utf8(bytes).unwrap().contains("SECRET_CANARY"));
}

#[test]
fn prompts_beyond_the_supported_host_limit_remain_a_finding() {
    let repo = TestRepo::new("plugin-prompt-limit");
    repo.write(
        ".codex-plugin/plugin.json",
        &serde_json::to_vec(&manifest(serde_json::json!([
            "one", "two", "three", "ignored"
        ])))
        .unwrap(),
    );
    repo.commit();

    let catalog = catalog(&repo);
    assert!(has_code(&catalog, "plugin_default_prompt_truncated"));
}

#[test]
fn declared_companions_must_exist_and_inline_servers_must_be_objects() {
    let repo = TestRepo::new("plugin-companions");
    let mut value = manifest(serde_json::json!(["Inspect this repository."]));
    value["apps"] = serde_json::json!("./.app.json");
    value["mcpServers"] = serde_json::json!({"unsafe": "not-an-object"});
    repo.write(
        ".codex-plugin/plugin.json",
        &serde_json::to_vec(&value).unwrap(),
    );
    repo.commit();

    let catalog = catalog(&repo);
    assert!(has_code(&catalog, "invalid_source_plugin_manifest"));
}

#[test]
fn component_paths_require_the_documented_dot_slash_prefix() {
    let repo = TestRepo::new("plugin-component-prefix");
    let mut value = manifest(serde_json::json!(["Inspect this repository."]));
    value["skills"] = serde_json::json!("skills/");
    repo.write(
        ".codex-plugin/plugin.json",
        &serde_json::to_vec(&value).unwrap(),
    );
    repo.commit();
    assert!(has_code(&catalog(&repo), "invalid_plugin_manifest_path"));
}

#[cfg(unix)]
#[test]
fn plugin_manifest_symlinks_are_never_cataloged() {
    let repo = TestRepo::new("plugin-manifest-symlink");
    std::fs::remove_file(repo.root.join(".codex-plugin/plugin.json")).unwrap();
    repo.write(
        "manifest-source.json",
        br#"{"name":"fixture-plugin","version":"1.0.0","description":"fixture"}"#,
    );
    std::os::unix::fs::symlink(
        repo.root.join("manifest-source.json"),
        repo.root.join(".codex-plugin/plugin.json"),
    )
    .unwrap();
    repo.commit();
    let catalog = catalog(&repo);
    assert!(has_code(&catalog, "plugin_manifest_symlink_unsupported"));
    assert!(
        !catalog
            .entries()
            .iter()
            .any(|entry| entry.stable_id == "PLUGIN-MANIFEST")
    );
}
