use serde_json::json;
use std::path::Path;

fn write_json(path: &Path, value: &serde_json::Value) {
    crate::self_tests::openai::write_json(path, value);
}

fn prepare_root(label: &str) -> std::path::PathBuf {
    crate::self_tests::openai::prepare_root(
        label,
        &[
            ".gitignore",
            "docs/openai-key-policy.json",
            "docs/openai-provider-policy.json",
            "validation_artifacts/openai/call-receipt.json",
            "validation_artifacts/openai/config-receipt.json",
            "validation_artifacts/openai/model-output-authority.json",
        ],
    )
}

fn write_config_and_call_receipts(root: &Path) {
    let config_args = ["openai", "config", "prove"]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let config_command = crate::cli::openai::parse(&config_args)
        .expect("parse config")
        .expect("config command");
    let config = crate::cli::openai::build_receipt(root, &config_command).expect("config");
    write_json(
        &root.join("validation_artifacts/openai/config-receipt.json"),
        &config,
    );

    let call_args = [
        "openai",
        "call",
        "prove",
        "--input-digest",
        &crate::self_tests::boundaries::workspace_fixtures::sha('a'),
        "--output-digest",
        &crate::self_tests::boundaries::workspace_fixtures::sha('b'),
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>();
    let call_command = crate::cli::openai::parse(&call_args)
        .expect("parse call")
        .expect("call command");
    let call = crate::cli::openai::build_receipt(root, &call_command).expect("call");
    write_json(
        &root.join("validation_artifacts/openai/call-receipt.json"),
        &call,
    );

    let output_args = [
        "openai",
        "output",
        "prove",
        "--parsed-output-digest",
        &crate::self_tests::boundaries::workspace_fixtures::sha('c'),
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>();
    let output_command = crate::cli::openai::parse(&output_args)
        .expect("parse output")
        .expect("output command");
    let output = crate::cli::openai::build_receipt(root, &output_command).expect("output");
    write_json(
        &root.join("validation_artifacts/openai/model-output-authority.json"),
        &output,
    );
}

#[test]
fn openai_package_audit_rejects_secret_shaped_receipt() {
    let root = prepare_root("openai-package-audit");
    write_config_and_call_receipts(&root);
    assert!(crate::audit::openai::package_failures(&root).is_empty());

    let mut receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/openai/config-receipt.json"),
    )
    .expect("receipt");
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
    write_config_and_call_receipts(&root);
    let mut receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/openai/config-receipt.json"),
    )
    .expect("receipt");
    receipt["candidate_digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('b'));
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

#[test]
fn openai_package_audit_rejects_wrong_call_candidate() {
    let root = prepare_root("openai-wrong-call-candidate");
    write_config_and_call_receipts(&root);
    let mut receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/openai/call-receipt.json"),
    )
    .expect("receipt");
    receipt["candidate_digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('c'));
    write_json(
        &root.join("validation_artifacts/openai/call-receipt.json"),
        &receipt,
    );
    let failures = crate::audit::openai::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item == "openai_call_receipt_candidate_digest_mismatch")
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn openai_package_audit_rejects_unbounded_call_retry_policy() {
    let root = prepare_root("openai-unbounded-retry");
    write_config_and_call_receipts(&root);
    let mut receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/openai/call-receipt.json"),
    )
    .expect("receipt");
    receipt["timeout_retry_backoff"]["max_retries"] = json!(99);
    write_json(
        &root.join("validation_artifacts/openai/call-receipt.json"),
        &receipt,
    );
    let failures = crate::audit::openai::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item == "openai_call_receipt_timeout_retry_backoff_invalid")
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn openai_package_audit_rejects_provider_policy_digest_mismatch() {
    let root = prepare_root("openai-provider-policy-mismatch");
    write_config_and_call_receipts(&root);
    let mut receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/openai/call-receipt.json"),
    )
    .expect("receipt");
    receipt["provider_policy_digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('d'));
    write_json(
        &root.join("validation_artifacts/openai/call-receipt.json"),
        &receipt,
    );
    let failures = crate::audit::openai::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item == "openai_call_receipt_provider_policy_digest_mismatch")
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn openai_package_audit_rejects_output_authority_overclaim() {
    let root = prepare_root("openai-output-overclaim");
    write_config_and_call_receipts(&root);
    let mut receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/openai/model-output-authority.json"),
    )
    .expect("receipt");
    receipt["authority_state"] = json!("deterministic_claim_authority");
    write_json(
        &root.join("validation_artifacts/openai/model-output-authority.json"),
        &receipt,
    );
    let failures = crate::audit::openai::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|item| item == "openai_model_output_receipt_authority_overbroad")
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
