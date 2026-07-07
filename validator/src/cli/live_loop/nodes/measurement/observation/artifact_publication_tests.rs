use super::*;
use crate::cli::live_loop::surfaces::surface_by_id;
use serde_json::json;

#[test]
fn malformed_observe_receipt_without_status_fails_with_typed_error() {
    let error = receipt_status(&json!({"schema":"observe-query"}), RoundtripQuery::Logs)
        .expect_err("missing receipt status");

    assert_eq!(error, "observe logs-query receipt missing status");
}

#[test]
fn observe_query_runner_propagates_command_failure() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-query-roundtrip-observe-command-failure",
    );
    std::fs::create_dir_all(&root).expect("root");
    let receipt = receipt_path("changed_files", "logs-query");

    let err = run_observe_query(
        &root,
        RoundtripQuery::Logs,
        receipt,
        "run-failure",
        "corr-failure",
    )
    .expect_err("observe command failure");

    assert!(
        err.contains("plugin-manifest-draft.json") || err.contains("package digest"),
        "{err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup observe command failure");
}

#[test]
fn query_run_propagates_explain_roundtrip_command_failure() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "live-loop-query-run-explain-command-failure",
    );
    std::fs::create_dir_all(&root).expect("root");
    let surface = surface_by_id("changed_files").expect("surface");

    let err = run(
        &root,
        surface,
        RoundtripQuery::ExplainFailure,
        "run-failure",
        "corr-failure",
        "sha256:missing-package-candidate",
    )
    .expect_err("public query run propagates explain failure");

    assert!(
        err.contains("plugin-manifest-draft.json") || err.contains("package digest"),
        "{err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup public run explain failure");
}

#[test]
fn observe_receipt_reader_reports_missing_and_malformed_receipts() {
    let root = prepared_root("live-loop-query-roundtrip-receipt-reader");
    let receipt = receipt_path("changed_files", "logs-query");

    let missing = read_observe_receipt(&root, &receipt, RoundtripQuery::Logs, 0)
        .expect_err("missing receipt");
    assert!(missing.contains("logs-query"), "{missing}");

    let absolute = root.join(&receipt);
    std::fs::create_dir_all(absolute.parent().expect("receipt parent")).expect("receipt parent");
    crate::json_boundary::write_json(&absolute, &json!({"schema":"observe-query"}))
        .expect("malformed receipt");
    let malformed = read_observe_receipt(&root, &receipt, RoundtripQuery::Logs, 0)
        .expect_err("malformed receipt");
    assert_eq!(malformed, "observe logs-query receipt missing status");

    std::fs::remove_dir_all(root).expect("cleanup receipt reader");
}

#[test]
fn observe_receipt_reader_rejects_external_claim_artifact_paths() {
    let root = prepared_root("live-loop-query-roundtrip-external-receipt");
    let err = read_observe_receipt(
        &root,
        std::path::Path::new("/tmp/live-loop-logs-query.json"),
        RoundtripQuery::Logs,
        0,
    )
    .expect_err("external receipt rejected");

    assert!(err.contains("external debug only"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup external receipt");
}

fn prepared_root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    root
}
