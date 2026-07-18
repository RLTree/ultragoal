#[test]
fn root_wiring_request_is_exact_but_non_authoritative() {
    let request: serde_json::Value =
        serde_json::from_str(&read("fixtures/plugin-product/root-wiring-request.json"))
            .unwrap_or_else(|error| panic!("root wiring request invalid: {error}"));
    assert_eq!(request["authoritative"], false);
    assert!(
        request["apply_only_after"]
            .as_str()
            .is_some_and(|value| value.contains("independent source acceptance"))
    );
    assert!(
        request["descriptor_membership_authority"]
            .as_str()
            .is_some_and(|value| value.contains("plugin-manifest-draft.json"))
    );
    assert!(
        request.get("root_owned_wiring_requests").is_none(),
        "withdrawn host-lifecycle and dispatcher requests must stay absent"
    );
    let request_json = request.to_string();
    let skills = request_json
        .split_once("\"pointer\":\"/skills\"")
        .unwrap_or_else(|| panic!("skills operation missing"))
        .1
        .split_once("\"pointer\":\"/purpose\"")
        .unwrap_or_else(|| panic!("purpose operation missing"))
        .0;
    for skill in SKILLS {
        assert_eq!(
            skills.matches(&format!("\"name\":\"{skill}\"")).count(),
            1,
            "root request skill {skill}"
        );
    }
    for agent in AGENTS {
        assert!(request_json.contains(&format!("\"{}\"", agent.name)));
        assert!(request_json.contains(&agent.path()));
    }
    assert_eq!(
        read(".codex-plugin/plugin.json").matches("0.0.12").count(),
        1
    );
    assert!(!root().join(".agents/plugins/marketplace.json").exists());
}
