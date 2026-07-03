use crate::audit::contract::Failure;
use serde_json::json;

fn errors(out: &[Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

#[test]
fn plugin_policy_rejects_custom_agent_boundary_and_runtime_drift() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("plugin-policy");
    std::fs::create_dir_all(root.join("custom-agents")).expect("custom agents");
    std::fs::write(root.join("custom-agents/unreadable.toml"), [0xff]).expect("bad utf8");
    std::fs::write(
        root.join("custom-agents/multiline.toml"),
        r#"
name = """multiline-string"""
description = "Specific reviewer"
developer_instructions = "Do the job."
"#,
    )
    .expect("multiline toml");
    std::fs::write(
        root.join("custom-agents/harness-product-simplicity-falsifier.toml"),
        r#"
name = "Wrong Visible Name"
description = ""
developer_instructions = ""
"#,
    )
    .expect("reviewer toml");
    let manifest = json!({
        "skills": [],
        "agents": [
            {
                "name": "harness-product-simplicity-falsifier",
                "path": "custom-agents/missing.toml",
                "app_visible_name": ""
            },
            {
                "name": "harness-contract-claim-falsifier",
                "path": "custom-agents/unreadable.toml",
                "app_visible_name": "Unreadable Agent"
            },
            {
                "name": "harness-missing-agent",
                "path": "custom-agents/missing.toml",
                "app_visible_name": "Missing Agent"
            },
            {
                "name": "harness-escape-agent",
                "path": "custom-agents/../escape.toml",
                "app_visible_name": "Escaping Agent"
            },
            {
                "name": "harness-multiline-agent",
                "path": "custom-agents/multiline.toml",
                "app_visible_name": "multiline-string"
            },
            {
                "name": "harness-security-trust-boundary-falsifier",
                "path": "custom-agents/harness-product-simplicity-falsifier.toml",
                "app_visible_name": "Harness Product Simplicity Falsifier"
            }
        ],
        "resources": []
    });
    let mut out = Vec::new();
    crate::claim_semantics::plugin_policy::check_plugin(
        &json!({"plugin_manifest": manifest}),
        &root,
        &mut out,
    );
    let got = errors(&out);
    assert!(got.contains(&"custom_agent_app_visible_name_missing"));
    assert!(got.contains(&"custom_agent_toml_unreadable"));
    assert!(got.contains(&"custom_agent_toml_field_missing"));
    assert!(
        out.iter()
            .any(|failure| failure.error == "custom_agent_toml_unreadable"
                && failure.detail.contains("missing.toml"))
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "custom_agent_toml_unreadable"
                && failure.detail.contains("escape.toml"))
    );
    assert!(got.contains(&"custom_agent_name_mismatch"));
    assert!(got.contains(&"custom_agent_reasoning_effort_not_high"));
    assert!(got.contains(&"custom_agent_sandbox_not_read_only"));
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
