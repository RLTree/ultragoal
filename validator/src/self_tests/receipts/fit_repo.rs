use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn has(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}

#[test]
fn fit_repo_receipt_reports_malformed_missing_and_unavailable_surfaces() {
    let root = crate::self_tests::boundaries::support::temp_root("fit-repo-missing");
    std::fs::create_dir_all(&root).expect("root");
    let failures = crate::audit::fit_repo_receipt::failures(
        &root,
        &json!({
            "target_revision":{"value":crate::self_tests::boundaries::support::sha('0')},
            "checks":[],
            "receipt_digest":crate::digest::ZERO
        }),
    );
    for expected in [
        "fit_repo_receipt_malformed:schema",
        "plugin_flow_entrypoint_missing",
        "fit_repo_receipt_target_digest_unavailable",
        "fit_repo_receipt_surface_missing:plugin_source_path",
        "fit_repo_receipt_placeholder_actor",
        "fit_repo_receipt_placeholder_digest",
        "fit_repo_receipt_unclassified_runtime_surface",
        "fit_repo_receipt_missing_check_result",
        "fit_repo_receipt_claim_ceiling_missing",
    ] {
        assert!(has(&failures, expected), "{expected}: {failures:?}");
    }
    std::fs::remove_dir_all(root).expect("cleanup missing");
}

#[test]
fn fit_repo_receipt_binds_version_cache_artifacts_and_canonical_digest() {
    let root = crate::self_tests::boundaries::support::temp_root("fit-repo-bound");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"version":"1.0.0","resources":[]}),
    );
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"version":"1.0.0"}),
    );
    std::fs::create_dir_all(root.join("validation_artifacts/harness")).expect("harness");
    std::fs::write(
        root.join("validation_artifacts/harness/fit-repo-command.stdout"),
        "stdout",
    )
    .expect("stdout");
    let mut receipt = json!({
        "schema":"harness-ultragoal.fit-repo-receipt.v1",
        "entrypoint_contract":{"id":"harness-ultragoal:fit-repo"},
        "target_revision":{"value":crate::self_tests::boundaries::support::sha('9')},
        "plugin_source_path":"source",
        "installed_plugin_path":"installed",
        "cache_package_path":"wrong-cache",
        "plugin_version":"0.9.0",
        "producer_actor_id":"fixture-author",
        "receipt_digest":crate::self_tests::boundaries::support::sha('1'),
        "target_classification":"blocked_unclassified_repo",
        "runtime_surface_classification":"",
        "product_surface_classification":"ambiguous_requires_blocker",
        "claim_ceiling":"",
        "checks":[{
            "stdout":{"path":"bad/stdout","digest":crate::self_tests::boundaries::support::sha('2')},
            "stderr":{"path":"validation_artifacts/harness/fit-repo-command.stdout","digest":crate::self_tests::boundaries::support::sha('3')}
        }],
        "blockers":[{}]
    });
    let failures = crate::audit::fit_repo_receipt::failures(&root, &receipt);
    for expected in [
        "fit_repo_receipt_target_digest_mismatch",
        "fit_repo_receipt_wrong_plugin_version",
        "fit_repo_receipt_wrong_cache_package",
        "fit_repo_receipt_placeholder_actor",
        "fit_repo_receipt_digest_mismatch",
        "fit_repo_receipt_unclassified_target",
        "fit_repo_receipt_unclassified_product_surface",
        "fit_repo_receipt_command_output_not_command_artifact",
        "fit_repo_receipt_artifact_invalid",
        "fit_repo_receipt_blocker_without_owner",
    ] {
        assert!(has(&failures, expected), "{expected}: {failures:?}");
    }
    let current = crate::package::inventory::package_digest(&root).expect("package digest");
    receipt["target_revision"]["value"] = json!(current);
    let rebound = crate::audit::fit_repo_receipt::failures(&root, &receipt);
    assert!(!has(&rebound, "fit_repo_receipt_target_digest_mismatch"));
    receipt["receipt_digest"] = json!(crate::audit::fit_repo_receipt::canonical_digest(&receipt));
    assert_eq!(
        receipt["receipt_digest"],
        json!(crate::audit::fit_repo_receipt::canonical_digest(&receipt))
    );
    std::fs::remove_dir_all(root).expect("cleanup bound");
}
