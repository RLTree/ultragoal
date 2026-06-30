use serde_json::json;
use std::path::Path;

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    crate::json_boundary::write_json(path, value).expect("write json");
}

fn prepare_root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root(label);
    std::fs::create_dir_all(root.join(".codex-worktree")).expect("worktree env dir");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join(".gitignore"), ".codex-worktree/\n").expect("gitignore");
    std::fs::write(
        root.join(".codex-worktree/env.sh"),
        "export OPENAI_API_KEY=test-key-redacted\n",
    )
    .expect("env");
    secure_env_file(&root.join(".codex-worktree/env.sh"));
    write_json(
        &root.join("docs/openai-key-policy.json"),
        &json!({
            "schema": "harness-ultragoal.openai-key-policy.v1",
            "law_id": crate::cli::openai::LAW_ID,
            "env_var": "OPENAI_API_KEY",
            "active_destination": ".codex-worktree/env.sh",
            "allowed_destinations": [".codex-worktree/env.sh"],
            "destination_must_be_gitignored": true,
            "unix_mode_required": "0600",
            "secret_serialization_policy": "forbid",
            "model_output_authority": "observation_only",
            "live_provider_requires": [
                "model_identity",
                "endpoint_api_family",
                "purpose",
                "prompt_input_digest",
                "output_digest",
                "schema_id",
                "timeout_retry_backoff",
                "token_cost_rate_limit_accounting",
                "candidate_digest",
                "run_id",
                "correlation_id",
                "claim_impact"
            ],
            "provider_modes": [
                "openai_live",
                "offline_fixture",
                "local_mock",
                "no_network"
            ],
            "claim_ceiling": "config_resolution_only"
        }),
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "resources": [".gitignore", "docs/openai-key-policy.json"],
            "schemas": [],
            "fixtures": [],
            "skills": [],
            "agents": [],
            "authorable_templates": [],
            "generated_examples": []
        }),
    );
    root
}

#[cfg(unix)]
fn secure_env_file(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(path, permissions).expect("mode");
}

#[cfg(not(unix))]
fn secure_env_file(_path: &Path) {}

#[test]
fn openai_config_receipt_redacts_secret_and_blocks_completion_claims() {
    let root = prepare_root("openai-config-receipt");
    let args = ["openai", "config", "prove"]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let command = crate::cli::openai::parse(&args)
        .expect("parse")
        .expect("openai command");
    let receipt = crate::cli::openai::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["secret_material_serialized"], false);
    assert_eq!(receipt["redaction_status"], "pass");
    assert_eq!(receipt["claim_ceiling"], "openai_config_resolution_only");
    let serialized = serde_json::to_string(&receipt).expect("serialize");
    assert!(!serialized.contains("test-key-redacted"), "{serialized}");
    assert!(!serialized.contains("OPENAI_API_KEY="), "{serialized}");
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|claim| claim.as_str() == Some("update_goal_eligibility"))
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn openai_config_receipt_fails_unapproved_destination() {
    let root = prepare_root("openai-bad-policy");
    write_json(
        &root.join("docs/openai-key-policy.json"),
        &json!({
            "schema": "harness-ultragoal.openai-key-policy.v1",
            "law_id": crate::cli::openai::LAW_ID,
            "env_var": "OPENAI_API_KEY",
            "active_destination": ".env",
            "allowed_destinations": [".env"],
            "destination_must_be_gitignored": true,
            "unix_mode_required": "0600",
            "secret_serialization_policy": "forbid",
            "model_output_authority": "observation_only",
            "live_provider_requires": [],
            "provider_modes": ["openai_live"],
            "claim_ceiling": "config_resolution_only"
        }),
    );
    let args = ["openai", "config", "prove"]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let command = crate::cli::openai::parse(&args)
        .expect("parse")
        .expect("openai command");
    let receipt = crate::cli::openai::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert!(
        receipt["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(
                |item| item.as_str() == Some("openai_key_policy_field_mismatch:active_destination")
            )
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn openai_call_receipt_blocks_live_model_claims() {
    let root = prepare_root("openai-call-receipt");
    let args = [
        "openai",
        "call",
        "prove",
        "--input-digest",
        &crate::self_tests::boundaries::support::sha('a'),
        "--output-digest",
        &crate::self_tests::boundaries::support::sha('b'),
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>();
    let command = crate::cli::openai::parse(&args)
        .expect("parse")
        .expect("openai call command");
    let receipt = crate::cli::openai::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["provider_mode"], "no_network");
    assert_eq!(
        receipt["model_output_authority"],
        "observation_only_until_cli_schema_validated"
    );
    assert_eq!(receipt["token_cost_rate_limit"]["total_tokens"], 0);
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|claim| claim.as_str() == Some("live_model_claim"))
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn openai_call_receipt_rejects_missing_digest_authority() {
    let root = prepare_root("openai-call-bad-digest");
    let args = [
        "openai",
        "call",
        "prove",
        "--input-digest",
        "not-a-digest",
        "--output-digest",
        &crate::self_tests::boundaries::support::sha('b'),
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>();
    let command = crate::cli::openai::parse(&args)
        .expect("parse")
        .expect("openai call command");
    let receipt = crate::cli::openai::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert!(
        receipt["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|item| item.as_str() == Some("openai_call_prompt_input_digest_invalid"))
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
