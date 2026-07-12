use serde_json::{Value, json};
use std::collections::BTreeMap;

#[test]
fn canonical_agent_manifest_closure_rejects_substitution_and_runtime_inference() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("agent-policy");
    write_canonical_agents(&root);
    let mut manifest = canonical_manifest();
    let mut out = Vec::new();
    super::required_agent_checks(&manifest, &root, &mut out);
    assert!(out.is_empty(), "{out:?}");

    manifest["agents"][0]["path"] = json!("custom-agents/claim-falsifier.toml");
    manifest["agents"].as_array_mut().unwrap().push(json!({
        "name":"seventh-role", "path":".codex/agents/seventh-role.toml"
    }));
    std::fs::write(
        root.join(".codex/agents/security-reviewer.toml"),
        agent_toml(
            "security-reviewer",
            "workspace-write",
            "model_reasoning_effort = \"high\"",
        ),
    )
    .expect("invalid canonical agent");
    out.clear();
    super::required_agent_checks(&manifest, &root, &mut out);
    let got = out
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(got.contains(&"canonical_agent_count_mismatch"));
    assert!(got.contains(&"canonical_agent_path_mismatch"));
    assert!(got.contains(&"unexpected_agent_role"));
    assert!(got.contains(&"canonical_agent_manifest_invalid"));

    std::fs::remove_file(root.join(".codex/agents/repo-recon.toml"))
        .expect("remove canonical agent");
    out.clear();
    super::required_agent_checks(&canonical_manifest(), &root, &mut out);
    assert!(out.iter().any(|failure| {
        failure.error == "canonical_agent_manifest_invalid"
            && failure.detail == ".codex/agents/repo-recon.toml"
    }));
    std::fs::remove_dir_all(root).expect("cleanup agent policy");
}

#[test]
fn legacy_agent_files_do_not_change_canonical_policy_results() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("legacy-agent-policy");
    write_canonical_agents(&root);
    let manifest = canonical_manifest();
    let mut before = Vec::new();
    super::required_agent_checks(&manifest, &root, &mut before);

    let legacy = root.join("custom-agents/harness-contract-claim-falsifier.toml");
    std::fs::create_dir_all(legacy.parent().unwrap()).expect("legacy root");
    std::fs::write(
        &legacy,
        agent_toml("claim-falsifier", "workspace-write", ""),
    )
    .expect("legacy bytes");
    let mut after = Vec::new();
    super::required_agent_checks(&manifest, &root, &mut after);
    assert_eq!(failure_triples(&before), failure_triples(&after));
    std::fs::write(legacy, "tampered").expect("tamper legacy bytes");
    after.clear();
    super::required_agent_checks(&manifest, &root, &mut after);
    assert_eq!(failure_triples(&before), failure_triples(&after));
    std::fs::remove_dir_all(root).expect("cleanup legacy agent policy");
}

#[test]
fn plugin_policy_cache_reuses_manifest_failure_sets() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-policy-cache");
    std::fs::create_dir_all(&root).expect("plugin policy cache root");
    let bundle = json!({"plugin_manifest":{"resources":["missing.txt"]}});
    let mut cache = BTreeMap::new();
    let mut first = Vec::new();
    super::check_plugin_with_cache(&bundle, &root, &mut first, &mut cache);
    assert_eq!(cache.len(), 1);
    let mut second = Vec::new();
    super::check_plugin_with_cache(&bundle, &root, &mut second, &mut cache);
    assert_eq!(failure_triples(&first), failure_triples(&second));
    std::fs::remove_dir_all(root).expect("cleanup plugin policy cache");
}

fn failure_triples(failures: &[crate::audit::contract::Failure]) -> Vec<(&str, &str, &str)> {
    failures
        .iter()
        .map(|failure| {
            (
                failure.check_id.as_str(),
                failure.error.as_str(),
                failure.detail.as_str(),
            )
        })
        .collect()
}

fn canonical_manifest() -> Value {
    json!({"agents": crate::agent_roles::CANONICAL_AGENT_ROLES.iter().map(|role| {
        json!({"name":role.name,"path":role.manifest_path})
    }).collect::<Vec<_>>()})
}

fn write_canonical_agents(root: &std::path::Path) {
    std::fs::create_dir_all(root.join(".codex/agents")).expect("agent root");
    for role in crate::agent_roles::CANONICAL_AGENT_ROLES {
        std::fs::write(
            root.join(role.manifest_path),
            agent_toml(role.name, "read-only", ""),
        )
        .expect("agent manifest");
    }
}

fn agent_toml(name: &str, sandbox: &str, extra: &str) -> String {
    format!(
        "name = \"{name}\"\ndescription = \"read only\"\ndeveloper_instructions = \"review\"\nsandbox_mode = \"{sandbox}\"\n{extra}\n"
    )
}
