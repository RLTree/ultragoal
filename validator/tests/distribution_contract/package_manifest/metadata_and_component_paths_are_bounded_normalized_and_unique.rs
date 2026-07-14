#[test]
fn metadata_and_component_paths_are_bounded_normalized_and_unique() {
    let fixture = Fixture::complete("package-manifest-paths");
    write_sources(&fixture);
    let spec = spec_value(false);
    for path in [
        "skills/",
        "./",
        "./../skills/",
        "./skills//",
        "./skills/./",
        "./skills\\x/",
        "./skills:x/",
        "./skills/\u{0}x/",
    ] {
        let mut value = manifest();
        value["skills"] = json!(path);
        assert_eq!(error(&fixture, &value, &spec), ErrorId::InvalidPath);
    }
    for invalid in ["1", "01.2.3", "1.2.3-01", "1.2.3+"] {
        let mut value = manifest();
        let mut bad_spec = spec.clone();
        value["version"] = json!(invalid);
        bad_spec["version"] = json!(invalid);
        assert_eq!(error(&fixture, &value, &bad_spec), ErrorId::InvalidSpec);
    }
    let mut aliases = manifest();
    aliases["interface"]["composerIcon"] = json!("./assets/icon.svg");
    aliases["interface"]["logo"] = json!("./assets/icon.svg");
    assert_eq!(error(&fixture, &aliases, &spec), ErrorId::InvalidSpec);
    aliases["interface"]["logo"] = json!("./Assets/icon.svg");
    assert_eq!(error(&fixture, &aliases, &spec), ErrorId::InvalidPath);
    let mut duplicate_keyword = manifest();
    duplicate_keyword["keywords"] = json!(["Proof", "proof"]);
    assert_eq!(
        error(&fixture, &duplicate_keyword, &spec),
        ErrorId::InvalidSpec
    );
}

#[test]
fn missing_empty_wrong_role_case_alias_and_unreferenced_content_fail() {
    let fixture = Fixture::complete("package-manifest-closure");
    write_sources(&fixture);
    let value = manifest();
    let mut empty = spec_value(false);
    empty["entries"].as_array_mut().unwrap().truncate(1);
    assert_eq!(error(&fixture, &value, &empty), ErrorId::ArchiveMismatch);
    let mut missing = manifest();
    missing["apps"] = json!("./.app.json");
    assert_eq!(
        error(&fixture, &missing, &spec_value(false)),
        ErrorId::ArchiveMismatch
    );
    let mut wrong = spec_value(false);
    wrong["entries"][1]["role"] = json!("data");
    assert_eq!(error(&fixture, &value, &wrong), ErrorId::ArchiveMismatch);
    for (role, executable) in [("data", false), ("executable", true)] {
        let mut extra = spec_value(false);
        add(&fixture, &mut extra, "unreferenced/bytes", role, executable);
        assert_eq!(error(&fixture, &value, &extra), ErrorId::ArchiveMismatch);
    }
    let mut alias = spec_value(false);
    alias["entries"][2]["path"] = json!("Skills/HARNESS-ULTRAGOAL/skill.md");
    assert_eq!(error(&fixture, &value, &alias), ErrorId::InvalidSpec);
}

#[test]
fn mcp_shapes_targets_roles_and_oversized_manifest_fail_closed() {
    let fixture = Fixture::complete("package-manifest-mcp-negative");
    write_sources(&fixture);
    for server in [
        json!({"type":"unknown","url":"https://mcp.terrynoblin.dev"}),
        json!({"type":"http","url":"https://mcp.terrynoblin.dev","args":["bad"]}),
        json!({"type":"stdio","command":"node","headers":{"X":"bad"}}),
        json!({"type":"http","url":"https://mcp.terrynoblin.dev","unknown":true}),
    ] {
        let mut value = manifest();
        value["mcpServers"] = json!({"server":server});
        assert_eq!(
            error(&fixture, &value, &spec_value(false)),
            ErrorId::InvalidSpec
        );
    }
    let mut file = manifest();
    file["mcpServers"] = json!("./.mcp.json");
    let mut file_spec = spec_value(false);
    add(&fixture, &mut file_spec, ".mcp.json", "data", false);
    write_manifest(&fixture, &file);
    plan_package(&fixture.root, &serde_json::to_vec(&file_spec).unwrap()).unwrap();
    let mut local = manifest();
    local["mcpServers"] = json!({"server":{"type":"stdio","command":"./bin/server"}});
    assert_eq!(
        error(&fixture, &local, &spec_value(false)),
        ErrorId::ArchiveMismatch
    );
    let mut wrong = spec_value(false);
    add(&fixture, &mut wrong, "bin/server", "data", false);
    assert_eq!(error(&fixture, &local, &wrong), ErrorId::ArchiveMismatch);
    fs::write(
        fixture.root.join("source/plugin.json"),
        vec![b' '; 1024 * 1024 + 1],
    )
    .unwrap();
    assert_eq!(
        plan_package(&fixture.root, &spec(false)).unwrap_err().id(),
        ErrorId::ObjectTooLarge
    );
}
