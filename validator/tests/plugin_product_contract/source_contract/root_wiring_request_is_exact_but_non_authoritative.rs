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
