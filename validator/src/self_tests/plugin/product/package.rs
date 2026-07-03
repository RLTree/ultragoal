use serde_json::json;

#[test]
fn plugin_product_package_failures_read_flow_fit_repo_and_journey_surfaces() {
    let empty =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-product-empty");
    std::fs::create_dir_all(&empty).expect("empty root");
    let missing = crate::audit::plugin::product::cohesion::package_failures(&empty);
    assert!(
        missing
            .iter()
            .any(|item| item.starts_with("plugin_product_surface_missing:")),
        "{missing:?}"
    );
    assert!(
        missing
            .iter()
            .any(|item| item.starts_with("plugin_flow_manifest_malformed:")),
        "{missing:?}"
    );
    std::fs::remove_dir_all(empty).expect("cleanup empty product root");

    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-product-package");
    for dir in [
        "skills/fit-repo",
        "schemas",
        "docs",
        "validation_artifacts/harness",
        ".codex-plugin",
        "templates/agent-standards",
        "custom-agents",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    for rel in [
        "skills/fit-repo/SKILL.md",
        "schemas/fit-repo-receipt.schema.json",
        "schemas/plugin-cohesion-manifest.schema.json",
        "custom-agents/harness-product-simplicity-falsifier.toml",
    ] {
        std::fs::write(root.join(rel), "surface\n").expect("surface");
    }
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({
            "skills":[{"path":"skills/fit-repo/SKILL.md"}],
            "schemas":["schemas/fit-repo-receipt.schema.json"],
            "authorable_templates":[],
            "agents":[{"path":"custom-agents/harness-product-simplicity-falsifier.toml"}]
        }))
        .expect("manifest"),
    )
    .expect("manifest");
    std::fs::write(
        root.join(".codex-plugin/plugin.json"),
        json!({"interface":{"defaultPrompt":["Use harness-ultragoal:fit-repo."]}}).to_string(),
    )
    .expect("plugin");
    std::fs::write(
        root.join("docs/plugin-resource-map.md"),
        "`ultragoal`\n`harness-ultragoal:fit-repo`\n",
    )
    .expect("resource map");
    std::fs::write(
        root.join("templates/agent-standards/enforcement.json"),
        json!({"rows":[{"id":"plugin-product-cohesion-authority"}]}).to_string(),
    )
    .expect("standards");
    std::fs::write(
        root.join("docs/plugin-cohesion-manifest.json"),
        json!({
            "schema":"harness-ultragoal.plugin-cohesion-manifest.v1",
            "entrypoints":["harness-ultragoal:fit-repo"],
            "edges":[{"from":"harness-ultragoal:fit-repo","to":"validation_artifacts/harness/fit-repo-receipt.json"}],
            "required_surfaces":["skills/fit-repo/SKILL.md"],
            "skills":["skills/fit-repo/SKILL.md"],
            "schemas":["schemas/fit-repo-receipt.schema.json"],
            "templates":[],
            "custom_agents":["custom-agents/harness-product-simplicity-falsifier.toml"],
            "setup_scripts":[],
            "fixture_groups":[],
            "receipts":[],
            "validator_checks":[],
            "standards_rows":["plugin-product-cohesion-authority"],
            "package_cache_install_surfaces":[]
        })
        .to_string(),
    )
    .expect("flow");
    std::fs::write(
        root.join("validation_artifacts/harness/fit-repo-receipt.json"),
        "{}",
    )
    .expect("fit-repo receipt");
    std::fs::write(
        root.join("validation_artifacts/harness/plugin-product-journey-receipt.json"),
        "{}",
    )
    .expect("journey receipt");
    let failures = crate::audit::plugin::product::cohesion::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item == "plugin_flow_entrypoint_not_primary"),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|item| item.starts_with("fit_repo_receipt_")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|item| item == "plugin_product_journey_incomplete"),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup product package root");
}
