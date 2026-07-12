use crate::audit::contract::Failure;
use serde_json::{Value, json};

fn errors(out: &[Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

#[test]
fn plugin_policy_rejects_noncanonical_agent_identity_and_static_write_authority() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-policy");
    write_canonical_agents(&root);
    std::fs::write(
        root.join(".codex/agents/security-reviewer.toml"),
        r#"
name = "security-reviewer"
description = "Specific reviewer."
developer_instructions = "Review and write anything."
sandbox_mode = "workspace-write"
model_reasoning_effort = "high"
"#,
    )
    .expect("invalid security reviewer");
    std::fs::remove_file(root.join(".codex/agents/repo-recon.toml"))
        .expect("missing canonical reviewer");
    let mut agents = canonical_agent_rows();
    agents[0]["path"] = json!("custom-agents/harness-contract-claim-falsifier.toml");
    agents.push(json!({
        "name":"seventh-role",
        "path":".codex/agents/seventh-role.toml"
    }));
    let manifest = json!({
        "skills": [],
        "agents": agents,
        "resources": []
    });
    let mut out = Vec::new();
    crate::claim_semantics::plugin_policy::check_plugin(
        &json!({"plugin_manifest": manifest}),
        &root,
        &mut out,
    );
    let got = errors(&out);
    assert!(got.contains(&"canonical_agent_count_mismatch"));
    assert!(got.contains(&"canonical_agent_path_mismatch"));
    assert!(got.contains(&"canonical_agent_manifest_invalid"));
    assert!(got.contains(&"unexpected_agent_role"));
    assert!(got.contains(&"required_skill_missing"));
    assert!(got.contains(&"plugin_agent_path_missing"));
    std::fs::remove_dir_all(root).expect("cleanup plugin policy");
}

#[test]
fn plugin_policy_reports_resource_purpose_skill_link_and_non_custom_agent_edges() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-policy-purpose");
    std::fs::create_dir_all(root.join("skills/bad")).expect("skill dir");
    std::fs::create_dir_all(root.join("references")).expect("root references");
    std::fs::create_dir_all(root.join("artifacts/root-receipts")).expect("artifacts dir");
    std::fs::write(root.join("references/missing.md"), "root-only reference\n").expect("reference");
    std::fs::write(
        root.join("skills/bad/SKILL.md"),
        "See [missing](references/missing.md)\n",
    )
    .expect("skill");
    std::fs::write(
        root.join("artifacts/fixture.json"),
        r#"{"fixture_only":true}"#,
    )
    .expect("fixture");
    let manifest = json!({
        "skills":[{"name":"bad-skill","path":"skills/bad/SKILL.md"}],
        "agents":[
            {"name":"non-custom","path":"agents/plain.md","app_visible_name":"Plain"}
        ],
        "resources":[
            "artifacts/stale-proof.json",
            "artifacts/root-receipts/post.json",
            "artifacts/fixture.json"
        ]
    });
    let mut out = Vec::new();
    crate::claim_semantics::plugin_policy::check_plugin(
        &json!({"plugin_manifest": manifest}),
        &root,
        &mut out,
    );
    let got = errors(&out);
    for expected in [
        "stale_artifact_resource_packaged",
        "root_verification_receipt_resource_packaged",
        "fixture_only_resource_packaged_as_active_artifact",
        "skill_local_reference_missing",
        "plugin_agent_path_missing",
    ] {
        assert!(got.contains(&expected), "{expected}: {out:?}");
    }
    std::fs::remove_dir_all(root).expect("cleanup plugin purpose policy");
}

fn canonical_agent_rows() -> Vec<Value> {
    crate::agent_roles::CANONICAL_AGENT_ROLES
        .iter()
        .map(|role| json!({"name":role.name,"path":role.manifest_path}))
        .collect()
}

fn write_canonical_agents(root: &std::path::Path) {
    for role in crate::agent_roles::CANONICAL_AGENT_ROLES {
        let path = root.join(role.manifest_path);
        std::fs::create_dir_all(path.parent().unwrap()).expect("agent root");
        std::fs::write(
            path,
            format!(
                "name = \"{}\"\ndescription = \"Read only.\"\ndeveloper_instructions = \"Review only.\"\nsandbox_mode = \"read-only\"\n",
                role.name
            ),
        )
        .expect("canonical agent");
    }
}
