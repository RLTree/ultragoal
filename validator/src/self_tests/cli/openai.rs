use serde_json::json;

fn prepare_root(label: &str) -> std::path::PathBuf {
    crate::self_tests::openai::prepare_root(
        label,
        &[
            ".gitignore",
            "docs/openai-key-policy.json",
            "docs/openai-provider-policy.json",
        ],
    )
}

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
    crate::self_tests::openai::write_json(
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
    assert_eq!(receipt["budget_class"], "source_no_network");
    assert_eq!(receipt["cache_policy"], "no_live_provider_cache");
    assert_eq!(
        receipt["model_output_authority"],
        "observation_only_until_cli_schema_validated"
    );
    assert_eq!(receipt["token_cost_rate_limit"]["total_tokens"], 0);
    assert_eq!(receipt["timeout_retry_backoff"]["max_retries"], 0);
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

#[test]
fn openai_output_receipt_dereferences_current_call_and_blocks_claims() {
    let root = prepare_root("openai-output-receipt");
    write_call_receipt(&root);
    let args = [
        "openai",
        "output",
        "prove",
        "--parsed-output-digest",
        &crate::self_tests::boundaries::support::sha('c'),
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>();
    let command = crate::cli::openai::parse(&args)
        .expect("parse")
        .expect("openai output command");
    let receipt = crate::cli::openai::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(
        receipt["authority_state"],
        "typed_observation_not_claim_authority"
    );
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|claim| claim.as_str() == Some("update_goal_eligibility"))
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn write_call_receipt(root: &std::path::Path) {
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
    let receipt = crate::cli::openai::build_receipt(root, &command).expect("receipt");
    crate::self_tests::openai::write_json(
        &root.join("validation_artifacts/openai/call-receipt.json"),
        &receipt,
    );
}
