use serde_json::json;
use std::path::Path;

pub(crate) fn prepare_root(label: &str, manifest_resources: &[&str]) -> std::path::PathBuf {
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
    write_key_policy(&root);
    write_provider_policy(&root);
    write_manifest(&root, manifest_resources);
    root
}

pub(crate) fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    crate::json_boundary::write_json(path, value).expect("write json");
}

fn write_key_policy(root: &Path) {
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
                "budget_class",
                "provider_policy_digest",
                "timeout_retry_backoff",
                "token_cost_rate_limit_accounting",
                "cache_policy",
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
}

fn write_provider_policy(root: &Path) {
    write_json(
        &root.join("docs/openai-provider-policy.json"),
        &json!({
            "schema": "harness-ultragoal.openai-provider-policy.v1",
            "law_id": crate::cli::openai::LAW_ID,
            "default_provider_mode": "no_network",
            "default_budget_class": "source_no_network",
            "allowed_provider_modes": ["openai_live", "offline_fixture", "local_mock", "no_network"],
            "budget_classes": budget_classes(),
            "forbidden_claim_substitutions": [
                "completion",
                "readiness",
                "release",
                "reviewer_exposure",
                "app_registry_exposure",
                "final_packet_correctness",
                "update_goal_eligibility",
                "product_success",
                "model_output_authority"
            ]
        }),
    );
}

fn budget_classes() -> serde_json::Value {
    json!({
        "source_no_network": budget_class(["no_network"], 0, 0, 0, "no_live_provider_cache"),
        "source_offline_fixture": budget_class(
            ["offline_fixture"],
            4096,
            2048,
            6144,
            "fixture_digest_only"
        ),
        "source_local_mock": budget_class(
            ["local_mock"],
            4096,
            2048,
            6144,
            "mock_digest_only"
        ),
        "source_live_low": {
            "provider_modes": ["openai_live"],
            "max_prompt_tokens": 8192,
            "max_completion_tokens": 2048,
            "max_total_tokens": 10240,
            "cost_ceiling_usd": 0.05,
            "timeout_ms": 30000,
            "max_retries": 2,
            "backoff_policy": "exponential_jitter",
            "backoff_initial_ms": 500,
            "backoff_max_ms": 4000,
            "cache_policy": "forbid_cache_for_live_claims",
            "no_cache_verification": "required_for_live_claims",
            "rate_limit_policy": "record_headers_or_unavailable_reason",
            "allowed_claim_ceiling": "live_model_observation_only"
        }
    })
}

fn budget_class<const N: usize>(
    modes: [&str; N],
    prompt: i64,
    completion: i64,
    total: i64,
    cache: &str,
) -> serde_json::Value {
    let provider_modes = modes.into_iter().collect::<Vec<_>>();
    json!({
        "provider_modes": provider_modes,
        "max_prompt_tokens": prompt,
        "max_completion_tokens": completion,
        "max_total_tokens": total,
        "cost_ceiling_usd": 0,
        "timeout_ms": 30000,
        "max_retries": 0,
        "backoff_policy": "none",
        "backoff_initial_ms": 0,
        "backoff_max_ms": 0,
        "cache_policy": cache,
        "no_cache_verification": format!("{cache}_digest_bound"),
        "rate_limit_policy": "not_applicable_without_live_provider",
        "allowed_claim_ceiling": "observation_only"
    })
}

fn write_manifest(root: &Path, resources: &[&str]) {
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "resources": resources,
            "schemas": [],
            "fixtures": [],
            "skills": [],
            "agents": [],
            "authorable_templates": [],
            "generated_examples": []
        }),
    );
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
