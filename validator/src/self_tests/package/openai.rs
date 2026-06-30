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
    std::fs::create_dir_all(root.join(".codex-worktree")).expect("env dir");
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
            "resources": [
                ".gitignore",
                "docs/openai-key-policy.json",
                "validation_artifacts/openai/config-receipt.json"
            ],
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
fn openai_package_audit_rejects_secret_shaped_receipt() {
    let root = prepare_root("openai-package-audit");
    let args = ["openai", "config", "prove"]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let command = crate::cli::openai::parse(&args)
        .expect("parse")
        .expect("openai command");
    let mut receipt = crate::cli::openai::build_receipt(&root, &command).expect("receipt");
    write_json(
        &root.join("validation_artifacts/openai/config-receipt.json"),
        &receipt,
    );
    assert!(crate::audit::openai::package_failures(&root).is_empty());

    receipt["debug_secret"] = json!(format!("{}-{}", "sk", "leak"));
    write_json(
        &root.join("validation_artifacts/openai/config-receipt.json"),
        &receipt,
    );
    let failures = crate::audit::openai::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item == "openai_config_receipt_secret_leak_or_redaction_failure")
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn openai_package_audit_rejects_wrong_candidate_receipt() {
    let root = prepare_root("openai-wrong-candidate");
    let args = ["openai", "config", "prove"]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let command = crate::cli::openai::parse(&args)
        .expect("parse")
        .expect("openai command");
    let mut receipt = crate::cli::openai::build_receipt(&root, &command).expect("receipt");
    receipt["candidate_digest"] = json!(crate::self_tests::boundaries::support::sha('b'));
    write_json(
        &root.join("validation_artifacts/openai/config-receipt.json"),
        &receipt,
    );
    let failures = crate::audit::openai::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item == "openai_config_receipt_candidate_digest_mismatch")
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
