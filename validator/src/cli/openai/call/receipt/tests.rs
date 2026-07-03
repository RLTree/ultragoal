use super::*;
use crate::self_tests::boundaries::workspace_fixtures::sha;
use serde_json::json;

#[test]
fn call_receipt_modes_distinguish_live_and_nonlive_authority() {
    let root = crate::self_tests::openai::prepare_root(
        "openai-call-receipt-modes",
        &[
            ".gitignore",
            "docs/openai-key-policy.json",
            "docs/openai-provider-policy.json",
        ],
    );
    let command = CallCommand {
        receipt: "validation_artifacts/openai/call.json".into(),
        provider_mode: "no_network".to_string(),
        model_identity: "none:no-network".to_string(),
        endpoint_api_family: "none:no-network".to_string(),
        purpose: "proof".to_string(),
        schema_id: "schema".to_string(),
        input_digest: sha('a'),
        output_digest: sha('b'),
        provider_policy: crate::cli::openai::budget::DEFAULT_POLICY.into(),
        budget_class: "source_no_network".to_string(),
    };
    let budget = crate::cli::openai::budget::load(
        &root,
        std::path::Path::new(crate::cli::openai::budget::DEFAULT_POLICY),
        "source_no_network",
        "no_network",
    );
    assert!(
        live_observation(&root, &command, &budget)
            .unwrap()
            .is_none()
    );
    assert_eq!(prompt_input_digest(&command, None), sha('a'));
    assert_eq!(output_digest(&command, None), sha('b'));
    assert_eq!(request_id(&command, None), "not_available:no_network");

    let live = live_observation_value();
    assert_eq!(prompt_input_digest(&command, Some(&live)), sha('c'));
    assert_eq!(output_digest(&command, Some(&live)), sha('d'));
    assert_eq!(request_id(&command, Some(&live)), "req_live");
    let cost = token_cost_rate_limit("candidate", &command, &budget, Some(&live));
    assert_eq!(cost["prompt_tokens"], 11);
    assert_eq!(cost["completion_tokens"], 13);
    assert_eq!(cost["total_tokens"], 24);
    assert_eq!(cost["rate_limit_observed"], true);
    assert_eq!(cost["rate_limit_unavailable_reason"], "none");
    assert_eq!(cost["latency_ms"], 321);
    assert!(receipt_failures("bad", "also-bad", &budget).len() >= 2);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn call_receipt_status_modes_cover_secret_downgrade() {
    let command = CallCommand {
        receipt: "receipt.json".into(),
        provider_mode: "offline_fixture".to_string(),
        model_identity: "fixture".to_string(),
        endpoint_api_family: "fixture".to_string(),
        purpose: "proof".to_string(),
        schema_id: "schema".to_string(),
        input_digest: sha('a'),
        output_digest: sha('b'),
        provider_policy: crate::cli::openai::budget::DEFAULT_POLICY.into(),
        budget_class: "source_offline_fixture".to_string(),
    };
    assert_eq!(request_id(&command, None), "not_available:offline_fixture");
    let local = CallCommand {
        provider_mode: "local_mock".to_string(),
        ..command
    };
    assert_eq!(request_id(&local, None), "not_available:local_mock");
    let live_uninvoked = CallCommand {
        provider_mode: "openai_live".to_string(),
        ..local
    };
    assert_eq!(
        request_id(&live_uninvoked, None),
        "not_available:openai_live_not_invoked"
    );
    let live = live_observation_value();
    let mut no_limit = live_observation_value();
    no_limit.rate_limit_observed = false;
    let root = crate::self_tests::openai::prepare_root(
        "openai-call-secret-downgrade",
        &["docs/openai-provider-policy.json"],
    );
    let budget = crate::cli::openai::budget::load(
        &root,
        std::path::Path::new(crate::cli::openai::budget::DEFAULT_POLICY),
        "source_no_network",
        "no_network",
    );
    assert_eq!(
        token_cost_rate_limit("candidate", &live_uninvoked, &budget, Some(&no_limit))["rate_limit_unavailable_reason"],
        "rate_limit_headers_not_returned"
    );
    let secret = secret_leak_receipt(json!({"status": "pass", "Authorization": "Bearer redacted"}));
    assert_eq!(secret["status"], "fail");
    assert_eq!(secret["redaction_status"], "fail");
    assert_eq!(
        secret["failures"][0],
        "openai_call_receipt_secret_shape_detected"
    );
    print_receipt(std::path::Path::new("receipt.json"), &json!({}));
    assert!(is_sha256(&sha('e')));
    assert!(!is_sha256("sha256:nothex"));
    drop(live);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn live_observation_failure_is_receipt_boundary_evidence() {
    let root = crate::self_tests::openai::prepare_root(
        "openai-live-observation-receipt",
        &[
            ".gitignore",
            "docs/openai-key-policy.json",
            "docs/openai-provider-policy.json",
        ],
    );
    let command = CallCommand {
        receipt: "receipt.json".into(),
        provider_mode: "openai_live".to_string(),
        model_identity: "gpt-5.4-nano".to_string(),
        endpoint_api_family: "responses".to_string(),
        purpose: "proof".to_string(),
        schema_id: "schema".to_string(),
        input_digest: sha('f'),
        output_digest: sha('g'),
        provider_policy: crate::cli::openai::budget::DEFAULT_POLICY.into(),
        budget_class: "source_live_low".to_string(),
    };
    let budget = crate::cli::openai::budget::load(
        &root,
        std::path::Path::new(crate::cli::openai::budget::DEFAULT_POLICY),
        "source_live_low",
        "openai_live",
    );
    let observation = live_observation(&root, &command, &budget)
        .expect("live observation")
        .expect("live mode observation");
    assert!(
        observation
            .failures
            .contains(&"openai_live_prompt_input_digest_mismatch".to_string())
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn openai_policy_loader_rejects_malformed_env_and_unapproved_paths() {
    let root = crate::self_tests::openai::prepare_root(
        "openai-policy-loader-edges",
        &["docs/openai-key-policy.json"],
    );
    crate::self_tests::openai::write_json(
        &root.join("docs/openai-key-policy.json"),
        &json!({
            "schema": "wrong",
            "law_id": "wrong",
            "env_var": "WRONG",
            "active_destination": "not-ignored/env.sh",
            "secret_serialization_policy": "allow",
            "model_output_authority": "claim_authority"
        }),
    );
    std::fs::create_dir_all(root.join("not-ignored")).expect("not ignored");
    std::fs::write(root.join("not-ignored/env.sh"), "sk-proj-redacted\n").expect("malformed env");
    let state = crate::cli::openai::policy::load(
        &root,
        std::path::Path::new(crate::cli::openai::config::DEFAULT_POLICY),
    );
    let failures = state.failures();
    assert!(failures.contains(&"openai_key_policy_unapproved_destination".to_string()));
    assert!(failures.contains(&"openai_key_destination_not_gitignored".to_string()));
    assert!(failures.contains(&"openai_key_destination_missing_openai_api_key".to_string()));
    assert!(failures.contains(&"openai_key_destination_malformed".to_string()));
    assert_eq!(
        crate::cli::openai::policy::api_key(
            &root,
            std::path::Path::new(crate::cli::openai::config::DEFAULT_POLICY),
        )
        .unwrap_err(),
        "openai_key_policy_not_passing"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn live_observation_value() -> crate::cli::openai::live::Observation {
    crate::cli::openai::live::Observation {
        prompt_input_digest: sha('c'),
        output_digest: sha('d'),
        request_id: "req_live".to_string(),
        prompt_tokens: 11,
        completion_tokens: 13,
        total_tokens: 24,
        latency_ms: 321,
        rate_limit_observed: true,
        failures: Vec::new(),
    }
}
