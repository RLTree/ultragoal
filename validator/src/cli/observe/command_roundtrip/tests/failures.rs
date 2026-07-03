use super::super::checks::roundtrip_path;
use super::super::process::CommandOutput;
use super::super::{receipt, roundtrip_status, run_command_roundtrip_with};
use super::{command, roundtrip_root};
use crate::audit::observability::specs;
use serde_json::json;
use std::path::Path;
use std::time::Instant;

fn successful_production(_: &Path, _: &[&str]) -> Result<CommandOutput, String> {
    Ok(CommandOutput {
        status_success: true,
        exit_code: 0,
        stdout: "ultragoal-package-digest pass".to_string(),
        stderr: String::new(),
    })
}

#[test]
fn command_roundtrip_stays_partial_without_live_query_roundtrip() {
    let (root, _) = roundtrip_root("observe-roundtrip-success");
    let spec = specs::command("package digest").expect("package spec");
    let row = run_command_roundtrip_with(&root, spec, 1, successful_production)
        .expect("local-spool-only roundtrip row");
    assert_eq!(row["roundtrip_status"], "partial");
    assert_eq!(row["stdout_receipt_same_candidate"], true);
    assert!(
        [
            "logs_query_status",
            "metrics_query_status",
            "traces_query_status"
        ]
        .into_iter()
        .any(|key| row[key] == "fail"),
        "{row}"
    );
    std::fs::remove_dir_all(root).expect("cleanup roundtrip success");
}

#[test]
fn roundtrip_status_labels_are_explicit() {
    assert_eq!(roundtrip_status(true), "observable");
    assert_eq!(roundtrip_status(false), "partial");
}

#[test]
fn command_roundtrip_fails_closed_when_production_command_cannot_run() {
    let (root, _) = roundtrip_root("observe-roundtrip-runner-error");
    let spec = specs::command("package digest").expect("package spec");
    let err = run_command_roundtrip_with(&root, spec, 1, |_, _| {
        Err("production command launch failed: denied".to_string())
    })
    .expect_err("runner failure");
    assert!(err.contains("production command launch failed"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup runner error");
}

#[test]
fn command_roundtrip_fails_closed_for_missing_or_malformed_command_receipts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-roundtrip-missing-receipt",
    );
    std::fs::create_dir_all(&root).expect("root");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["plugin-manifest-draft.json"]}),
    )
    .expect("manifest");
    let spec = specs::command("package digest").expect("package spec");
    let err = run_command_roundtrip_with(&root, spec, 1, successful_production)
        .expect_err("missing command receipt");
    assert!(err.contains("package-digest.json"), "{err}");

    let (missing_run, candidate) = roundtrip_root("observe-roundtrip-missing-run-id");
    crate::json_boundary::write_json(
        &missing_run.join("validation_artifacts/observability/package-digest.json"),
        &json!({"candidate_digest": candidate, "correlation_id": "corr-fit", "status": "pass"}),
    )
    .expect("receipt without run id");
    let err = run_command_roundtrip_with(&missing_run, spec, 1, successful_production)
        .expect_err("missing run id");
    assert!(err.contains("missing run_id"), "{err}");

    let (missing_corr, candidate) = roundtrip_root("observe-roundtrip-missing-correlation-id");
    crate::json_boundary::write_json(
        &missing_corr.join("validation_artifacts/observability/package-digest.json"),
        &json!({"candidate_digest": candidate, "run_id": "run-fit", "status": "pass"}),
    )
    .expect("receipt without correlation id");
    let err = run_command_roundtrip_with(&missing_corr, spec, 1, successful_production)
        .expect_err("missing correlation id");
    assert!(err.contains("missing correlation_id"), "{err}");
}

#[test]
fn command_roundtrip_fails_closed_when_query_or_explain_receipts_cannot_write() {
    let spec = specs::command("package digest").expect("package spec");
    for (label, suffix, expected) in [
        ("logs", "logs", "package-digest-logs.json"),
        ("metrics", "metrics", "package-digest-metrics.json"),
        ("traces", "traces", "package-digest-traces.json"),
        (
            "explain",
            "explain-failure",
            "package-digest-explain-failure.json",
        ),
    ] {
        let (root, _) = roundtrip_root(&format!("observe-roundtrip-{label}-write-error"));
        std::fs::create_dir_all(root.join(roundtrip_path(spec, suffix)))
            .expect("block roundtrip path");
        let err = run_command_roundtrip_with(&root, spec, 1, successful_production)
            .expect_err("roundtrip receipt path rejects directory");
        assert!(err.contains(expected), "{label}: {err}");
        std::fs::remove_dir_all(root).expect("cleanup proof write error");
    }
}

#[test]
fn command_roundtrip_receipt_fails_closed_when_base_observability_cannot_emit() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "observe-roundtrip-receipt-error",
    );
    std::fs::create_dir_all(&root).expect("root");
    let err = receipt(
        &root,
        &command(),
        "sha256:fit".to_string(),
        vec![json!({"roundtrip_status": "observable"})],
        Instant::now(),
    )
    .expect_err("base receipt needs package boundary");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
}
