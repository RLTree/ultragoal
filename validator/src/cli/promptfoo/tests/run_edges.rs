use super::{fake_promptfoo, fake_promptfoo_failing, seed_root};
use crate::cli::promptfoo::{PromptfooCommand, proof, registry};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn promptfoo_run_parse_registry_and_tamper_edges_are_typed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("promptfoo-run");
    seed_root(&root);
    let receipt_rel = "validation_artifacts/promptfoo/run-receipt.json";
    let receipt_path = root.join(receipt_rel);
    let bin = fake_promptfoo(&root);
    let command = parse_promptfoo(&[
        "promptfoo",
        "prove",
        "--receipt",
        receipt_rel,
        "--promptfoo-bin",
        bin.to_str().expect("bin path"),
    ]);
    assert_promptfoo_parse_failures();
    assert_eq!(
        crate::cli::promptfoo::run(&root, &command).expect("run promptfoo"),
        0
    );
    let receipt = crate::json_boundary::read_json(&receipt_path).expect("receipt json");
    assert!(crate::cli::promptfoo::receipt_failures(&root, &receipt).is_empty());

    assert_missing_and_failing_promptfoo(&root);
    assert_missing_promptfoo_json_dependency(&root, &bin);
    assert_secret_and_shape_failures(&root);
    assert_registry_failures();
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn assert_missing_promptfoo_json_dependency(root: &std::path::Path, bin: &std::path::Path) {
    let provider = root.join(crate::cli::promptfoo::PROVIDER_REGISTRY);
    let saved = crate::json_boundary::read_json(&provider).expect("provider");
    std::fs::write(&provider, "{").expect("malformed provider");
    let receipt = proof::build_receipt(
        root,
        &PromptfooCommand {
            receipt: PathBuf::from(crate::cli::promptfoo::DEFAULT_RECEIPT),
            promptfoo_bin: bin.to_path_buf(),
        },
    )
    .expect("missing provider receipt");
    assert!(
        receipt["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|failure| failure
                .as_str()
                .is_some_and(|text| text.starts_with("promptfoo_json_missing_or_malformed:"))),
        "{receipt}"
    );
    crate::json_boundary::write_json(&provider, &saved).expect("restore provider");
}

fn parse_promptfoo(args: &[&str]) -> PromptfooCommand {
    crate::cli::promptfoo::parse(
        &args
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>(),
    )
    .expect("parse promptfoo")
    .expect("promptfoo command")
}

fn assert_promptfoo_parse_failures() {
    assert!(
        crate::cli::promptfoo::parse(&["not-promptfoo".to_string()])
            .expect("non promptfoo")
            .is_none()
    );
    assert!(crate::cli::promptfoo::parse(&["promptfoo".to_string()]).is_err());
}

fn assert_missing_and_failing_promptfoo(root: &std::path::Path) {
    let missing = proof::build_receipt(
        root,
        &PromptfooCommand {
            receipt: PathBuf::from(crate::cli::promptfoo::DEFAULT_RECEIPT),
            promptfoo_bin: PathBuf::from("missing-promptfoo"),
        },
    )
    .expect("missing promptfoo");
    assert_eq!(missing["status"], "fail");
    assert_version_failure(&missing, "promptfoo_version_command_unavailable:");

    let failing_bin = fake_promptfoo_failing(root);
    let failing = proof::build_receipt(
        root,
        &PromptfooCommand {
            receipt: PathBuf::from(crate::cli::promptfoo::DEFAULT_RECEIPT),
            promptfoo_bin: failing_bin,
        },
    )
    .expect("failing promptfoo");
    assert_version_failure(&failing, "promptfoo_version_command_failed:");
}

fn assert_version_failure(receipt: &serde_json::Value, prefix: &str) {
    assert!(
        receipt["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|failure| failure
                .as_str()
                .is_some_and(|text| text.starts_with(prefix)))
    );
}

fn assert_secret_and_shape_failures(root: &std::path::Path) {
    let secret = proof::build_receipt(
        root,
        &PromptfooCommand {
            receipt: PathBuf::from(crate::cli::promptfoo::DEFAULT_RECEIPT),
            promptfoo_bin: PathBuf::from("Authorization: Bearer redacted"),
        },
    )
    .expect("secret promptfoo");
    assert_eq!(secret["status"], "fail");
    assert_eq!(
        secret["failures"][0],
        "promptfoo_receipt_secret_shape_detected"
    );

    let bad = crate::cli::promptfoo::receipt_failures(root, &json!({}));
    for expected in [
        "promptfoo_receipt_field_mismatch:schema",
        "promptfoo_receipt_field_mismatch:status",
        "promptfoo_receipt_field_mismatch:candidate_digest",
        "promptfoo_receipt_field_mismatch:promptfoo_version",
        "promptfoo_receipt_field_mismatch:raw_promptfoo_authority",
        "promptfoo_receipt_missing_claim_blockers",
        "promptfoo_receipt_observability_binding_invalid",
    ] {
        assert!(bad.contains(&expected.to_string()), "{expected}: {bad:?}");
    }
    let secret_failures =
        crate::cli::promptfoo::receipt_failures(root, &json!({"Authorization": "Bearer redacted"}));
    assert!(secret_failures.contains(&"promptfoo_receipt_secret_shape_detected".to_string()));
}

fn assert_registry_failures() {
    let package_failures = registry::package_failures(&json!({}));
    assert!(package_failures.contains(&"promptfoo_package_json_not_pinned".to_string()));
    assert!(package_failures.contains(&"promptfoo_package_manager_not_pinned".to_string()));
    let provider_failures = registry::provider_failures(&json!({}));
    assert!(
        provider_failures
            .iter()
            .any(|failure| failure == "promptfoo_registry_field_mismatch:/schema")
    );
    assert!(
        provider_failures
            .iter()
            .any(|failure| failure.starts_with("promptfoo_provider_mode_missing:"))
    );
    assert!(
        provider_failures
            .iter()
            .any(|failure| failure.starts_with("promptfoo_forbidden_substitution_missing:"))
    );
    let suite_failures = registry::suite_failures(&json!({}));
    assert!(suite_failures.contains(&"promptfoo_suite_smoke_missing".to_string()));
}
