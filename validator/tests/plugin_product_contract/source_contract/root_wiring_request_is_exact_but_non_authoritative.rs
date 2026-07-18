#[test]
fn root_wiring_request_is_exact_but_non_authoritative() {
    let request = read("fixtures/plugin-product/root-wiring-request.json");
    assert!(request.contains("\"authoritative\": false"));
    assert!(request.contains("\"value\": \"0.0.12\""));
    assert!(request.contains("./plugins/harness-ultragoal"));
    assert!(request.contains("path_absent"));
    let skills = request
        .split_once("\"pointer\": \"/skills\"")
        .unwrap_or_else(|| panic!("skills operation missing"))
        .1
        .split_once("\"pointer\": \"/purpose\"")
        .unwrap_or_else(|| panic!("purpose operation missing"))
        .0;
    for skill in SKILLS {
        assert_eq!(
            skills.matches(&format!("\"name\": \"{skill}\"")).count(),
            1,
            "root request skill {skill}"
        );
    }
    for agent in AGENTS {
        assert!(request.contains(&format!("\"{}\"", agent.name)));
        assert!(request.contains(&agent.path()));
    }
    assert_eq!(
        read(".codex-plugin/plugin.json").matches("0.0.11").count(),
        1
    );
    assert!(!root().join(".agents/plugins/marketplace.json").exists());
}

#[test]
fn supported_host_wiring_is_root_owned_and_preserves_each_proof_surface() {
    let request: serde_json::Value =
        serde_json::from_str(&read("fixtures/plugin-product/root-wiring-request.json"))
            .unwrap_or_else(|error| panic!("root wiring request invalid: {error}"));
    let rows = request["root_owned_wiring_requests"]
        .as_array()
        .unwrap_or_else(|| panic!("root-owned wiring requests missing"));
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows[0]["path"], "validator/src/plugin_product/mod.rs",
        "only root may compile the internal host lifecycle module"
    );
    assert!(
        rows[0]["required_guard"]
            .as_str()
            .is_some_and(|value| value.contains("externally public"))
    );
    assert_eq!(rows[1]["path"], "validator/src/cli/successor_public/mod.rs");
    assert!(
        rows[1]["requested_change"]
            .as_str()
            .is_some_and(|value| value.contains("root-issued distribution effect permit"))
    );
    assert!(
        rows[2]["required_guard"]
            .as_str()
            .is_some_and(|value| value.contains("same-surface proof"))
    );
}
