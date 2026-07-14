pub fn manifest() -> Value {
    json!({
        "name": PLUGIN_ID,
        "version": VERSION,
        "description": "Repository fit, routine work, diagnosis, proof, and migration.",
        "author": {
            "name": "Terry Noblin",
            "email": "tree@terrynoblin.dev",
            "url": "https://terrynoblin.dev"
        },
        "homepage": "https://terrynoblin.dev/harness-ultragoal",
        "repository": "https://github.com/terrynoblin/harness-ultragoal",
        "license": "UNLICENSED",
        "keywords": ["agent-first", "verification"],
        "skills": "./skills/",
        "interface": {
            "displayName": "Harness Ultragoal",
            "shortDescription": "One evidence-bound front door.",
            "longDescription": "Route repository work through explicit authority and effects.",
            "developerName": "Terry Noblin",
            "category": "Productivity",
            "capabilities": ["Read", "Write"],
            "websiteURL": "https://terrynoblin.dev/harness-ultragoal",
            "defaultPrompt": ["Classify this repository task and route it safely."],
            "brandColor": "#3B82F6"
        }
    })
}

pub fn spec(reverse: bool) -> Vec<u8> {
    serde_json::to_vec(&spec_value(reverse)).unwrap()
}

pub fn write_sources(fixture: &Fixture) {
    fs::create_dir_all(fixture.root.join("source")).unwrap();
    fs::write(
        fixture.root.join("source/plugin.json"),
        serde_json::to_vec(&manifest()).unwrap(),
    )
    .unwrap();
    fs::write(
        fixture.root.join("source/skill-one.md"),
        b"---\nname: harness-ultragoal\ndescription: Harness front door\n---\n",
    )
    .unwrap();
    fs::write(
        fixture.root.join("source/skill-two.md"),
        b"---\nname: prove\ndescription: Proof workflow\n---\n",
    )
    .unwrap();
}

pub(crate) fn spec_value(reverse: bool) -> Value {
    let mut entries = vec![
        row(
            ".codex-plugin/plugin.json",
            "source/plugin.json",
            "manifest",
            false,
        ),
        row(
            "skills/harness-ultragoal/SKILL.md",
            "source/skill-one.md",
            "skill",
            false,
        ),
        row(
            "skills/prove/SKILL.md",
            "source/skill-two.md",
            "skill",
            false,
        ),
    ];
    if reverse {
        entries.reverse();
    }
    json!({
        "schema": "harness-ultragoal.package-plan.v1",
        "context_id": CONTEXT_ID,
        "candidate_id": CANDIDATE_ID,
        "plugin_id": PLUGIN_ID,
        "version": VERSION,
        "source_date_epoch": 1_700_000_000u64,
        "entries": entries
    })
}

fn row(path: &str, source: &str, role: &str, executable: bool) -> Value {
    json!({"path":path,"source_path":source,"role":role,"executable":executable})
}

pub(crate) fn add(fixture: &Fixture, spec: &mut Value, path: &str, role: &str, executable: bool) {
    let source = format!("source/extra-{}", spec["entries"].as_array().unwrap().len());
    fs::write(fixture.root.join(&source), format!("fixture for {path}\n")).unwrap();
    spec["entries"]
        .as_array_mut()
        .unwrap()
        .push(row(path, &source, role, executable));
}

pub(crate) fn write_manifest(fixture: &Fixture, value: &Value) {
    fs::write(
        fixture.root.join("source/plugin.json"),
        serde_json::to_vec(value).unwrap(),
    )
    .unwrap();
}

pub(crate) fn error(fixture: &Fixture, value: &Value, spec: &Value) -> ErrorId {
    write_manifest(fixture, value);
    plan_package(&fixture.root, &serde_json::to_vec(spec).unwrap())
        .unwrap_err()
        .id()
}

#[test]
fn realistic_supported_components_close_and_order_is_deterministic() {
    let fixture = Fixture::complete("package-manifest-positive");
    write_sources(&fixture);
    let mut value = manifest();
    value["apps"] = json!("./.app.json");
    value["mcpServers"] = json!({
        "local-server": {"type":"stdio","command":"./bin/server","args":["--serve"],"env":{"MODE":"test"}},
        "remote-server": {"type":"http","url":"https://mcp.terrynoblin.dev","headers":{"X-Mode":"test"}}
    });
    value["interface"]["composerIcon"] = json!("./assets/icon.svg");
    value["interface"]["screenshots"] = json!(["./assets/overview.png"]);
    write_manifest(&fixture, &value);
    let mut first = spec_value(false);
    for (path, role, executable) in [
        (".app.json", "data", false),
        ("bin/server", "executable", true),
        ("assets/icon.svg", "data", false),
        ("assets/overview.png", "data", false),
        ("skills/x/scripts/a", "executable", true),
        ("skills/x/assets/a", "data", false),
        ("skills/x/references/a", "documentation", false),
        ("skills/x/agents/a", "agent", false),
    ] {
        add(&fixture, &mut first, path, role, executable);
    }
    let before = tree(&fixture.root);
    let one = plan_package(&fixture.root, &serde_json::to_vec(&first).unwrap()).unwrap();
    let mut wrong = first.clone();
    wrong["entries"][7]["role"] = json!("data");
    wrong["entries"][7]["executable"] = json!(false);
    assert_eq!(error(&fixture, &value, &wrong), ErrorId::ArchiveMismatch);
    first["entries"].as_array_mut().unwrap().reverse();
    let two = plan_package(&fixture.root, &serde_json::to_vec(&first).unwrap()).unwrap();
    assert_eq!(one.entries(), two.entries());
    assert_eq!(tree(&fixture.root), before);
}

#[test]
fn name_version_only_unknown_duplicate_and_placeholder_manifests_fail() {
    let fixture = Fixture::complete("package-manifest-typed");
    write_sources(&fixture);
    let spec = spec_value(false);
    let mut missing = manifest();
    missing.as_object_mut().unwrap().remove("description");
    assert_eq!(error(&fixture, &missing, &spec), ErrorId::InvalidSpec);
    let mut unknown = manifest();
    unknown["hooks"] = json!({"SessionStart":[]});
    assert_eq!(error(&fixture, &unknown, &spec), ErrorId::InvalidSpec);
    let mut nested = manifest();
    nested["interface"]["unsupported"] = json!(true);
    assert_eq!(error(&fixture, &nested, &spec), ErrorId::InvalidSpec);
    for description in ["", "TODO", "[TODO: fill me]", "bad\u{0}value"] {
        let mut value = manifest();
        value["description"] = json!(description);
        assert_eq!(error(&fixture, &value, &spec), ErrorId::InvalidSpec);
    }
    fs::write(
        fixture.root.join("source/plugin.json"),
        br#"{"name":"harness-ultragoal","name":"harness-ultragoal","version":"0.0.11","description":"valid","skills":"./skills/"}"#,
    )
    .unwrap();
    assert_eq!(
        plan_package(&fixture.root, &serde_json::to_vec(&spec).unwrap())
            .unwrap_err()
            .id(),
        ErrorId::InvalidJson
    );
}
