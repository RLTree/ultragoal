use super::*;
use crate::self_tests::boundaries::workspace_fixtures::sha;
use serde_json::json;

#[test]
fn call_parse_run_and_live_defaults_are_typed() {
    let live = parse(&[
        "openai".to_string(),
        "call".to_string(),
        "prove".to_string(),
        "--mode".to_string(),
        "openai_live".to_string(),
        "--input-digest".to_string(),
        sha('a'),
        "--output-digest".to_string(),
        sha('b'),
    ])
    .expect("parse live");
    assert_eq!(live.model_identity, "gpt-5.4-nano");
    assert_eq!(live.endpoint_api_family, "responses");
    let defaults = parse(&[
        "openai".to_string(),
        "call".to_string(),
        "prove".to_string(),
    ])
    .expect("parse defaults");
    assert_eq!(defaults.input_digest, crate::digest::ZERO);
    assert_eq!(defaults.output_digest, crate::digest::ZERO);
    assert_eq!(default_model("no_network"), "none:no-network");
    assert_eq!(default_endpoint("no_network"), "none:no-network");

    let root = crate::self_tests::openai::prepare_root(
        "openai-call-run-fail",
        &["docs/openai-provider-policy.json"],
    );
    let bad = parse(&[
        "openai".to_string(),
        "call".to_string(),
        "prove".to_string(),
        "--input-digest".to_string(),
        "not-a-digest".to_string(),
    ])
    .expect("parse bad call");
    assert_eq!(run(&root, &bad).expect("run bad call"), 1);

    let live_bad = parse(&[
        "openai".to_string(),
        "call".to_string(),
        "prove".to_string(),
        "--mode".to_string(),
        "openai_live".to_string(),
        "--input-digest".to_string(),
        sha('a'),
        "--output-digest".to_string(),
        sha('b'),
    ])
    .expect("parse live bad call");
    let live_receipt = build_call_receipt(&root, &live_bad).expect("live receipt");
    assert_eq!(live_receipt["status"], "fail");
    assert!(
        !live_receipt["failures"]
            .as_array()
            .expect("failures")
            .is_empty()
    );

    let secret = parse(&[
        "openai".to_string(),
        "call".to_string(),
        "prove".to_string(),
        "--model".to_string(),
        "Authorization: Bearer redacted".to_string(),
        "--input-digest".to_string(),
        sha('a'),
        "--output-digest".to_string(),
        sha('b'),
    ])
    .expect("parse secret call");
    let secret_receipt = build_call_receipt(&root, &secret).expect("secret receipt");
    assert_eq!(secret_receipt["status"], "fail");
    assert_eq!(
        secret_receipt["failures"][0],
        "openai_call_receipt_secret_shape_detected"
    );
    let spool = root.join("validation_artifacts/observability/spool");
    let _ = std::fs::remove_dir_all(&spool);
    std::fs::write(&spool, "not a directory").expect("spool blocker");
    let err = build_call_receipt(&root, &bad).expect_err("spool blocker must fail");
    assert!(err.contains("validation_artifacts/observability/spool"));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn openai_budget_policy_and_config_edges_are_typed() {
    assert_eq!(
        crate::cli::openai::budget::default_class("offline_fixture"),
        "source_offline_fixture"
    );
    assert_eq!(
        crate::cli::openai::budget::default_class("local_mock"),
        "source_local_mock"
    );
    assert_eq!(
        crate::cli::openai::budget::default_class("unknown"),
        "source_no_network"
    );

    let root = crate::self_tests::openai::prepare_root(
        "openai-budget-config-edges",
        &[
            "docs/openai-key-policy.json",
            "docs/openai-provider-policy.json",
        ],
    );
    let abs = root.join(crate::cli::openai::budget::DEFAULT_POLICY);
    let selection =
        crate::cli::openai::budget::load(&root, &abs, "source_no_network", "no_network");
    assert!(selection.failures.is_empty(), "{:?}", selection.failures);
    assert_eq!(
        crate::cli::openai::budget::cache_policy(&selection),
        "no_live_provider_cache"
    );

    let bad_policy = root.join("docs/openai-provider-policy-bad.json");
    crate::self_tests::openai::write_json(
        &bad_policy,
        &json!({
            "schema": "wrong",
            "law_id": "wrong",
            "allowed_provider_modes": [],
            "budget_classes": {
                "bad": {
                    "provider_modes": ["no_network"],
                    "timeout_ms": 1,
                    "max_retries": 4
                }
            }
        }),
    );
    let bad = crate::cli::openai::budget::load(&root, &bad_policy, "bad", "no_network");
    assert!(
        bad.failures
            .contains(&"openai_provider_policy_wrong_schema".to_string())
    );
    assert!(
        bad.failures
            .contains(&"openai_provider_budget_retry_invalid".to_string())
    );
    let bad_cost = crate::cli::openai::budget::cost_rate_limit("candidate", "no_network", &bad);
    assert_eq!(bad_cost["status"], "fail");
    assert_eq!(bad_cost["cost_unavailable_reason"], "no_provider_call");
    assert_eq!(
        bad_cost["rate_limit_unavailable_reason"],
        "no_provider_call"
    );
    let offline_cost =
        crate::cli::openai::budget::cost_rate_limit("candidate", "offline_fixture", &bad);
    assert_eq!(
        offline_cost["cost_unavailable_reason"],
        "offline_or_mock_provider"
    );

    let secret_policy = root.join("docs/sk-policy.json");
    crate::self_tests::openai::write_json(
        &secret_policy,
        &json!({
            "schema": "harness-ultragoal.openai-key-policy.v1",
            "law_id": crate::cli::openai::LAW_ID,
            "env_var": "OPENAI_API_KEY",
            "active_destination": ".codex-worktree/env.sh",
            "secret_serialization_policy": "forbid",
            "model_output_authority": "observation_only",
            "leaked_value": "sk-proj-redacted"
        }),
    );
    let receipt = crate::cli::openai::config::build_config_receipt(
        &root,
        &crate::cli::openai::config::ConfigCommand {
            receipt: "validation_artifacts/openai/config.json".into(),
            policy: secret_policy.clone(),
        },
    )
    .expect("config receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["failures"][0],
        "openai_config_receipt_secret_shape_detected"
    );

    let secret_state = crate::cli::openai::policy::load(&root, &secret_policy);
    assert!(
        secret_state
            .failures()
            .contains(&"openai_key_policy_secret_shape_detected".to_string())
    );

    let spool = root.join("validation_artifacts/observability/spool");
    let _ = std::fs::remove_dir_all(&spool);
    std::fs::write(&spool, "not a directory").expect("spool blocker");
    let err = crate::cli::openai::config::build_config_receipt(
        &root,
        &crate::cli::openai::config::ConfigCommand {
            receipt: "validation_artifacts/openai/config-spool.json".into(),
            policy: crate::cli::openai::config::DEFAULT_POLICY.into(),
        },
    )
    .expect_err("spool blocker must fail config telemetry");
    assert!(err.contains("validation_artifacts/observability/spool"));
    std::fs::remove_dir_all(root).expect("cleanup");
}
