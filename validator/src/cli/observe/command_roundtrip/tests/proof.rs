use super::super::checks::{query_paths, roundtrip_path, status, text};
use super::super::{explain_roundtrip, query_roundtrip};
use super::roundtrip_root;
use crate::audit::observability::specs;
use crate::cli::observe::query::QueryKind;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn check_helpers_build_paths_report_status_and_required_text() {
    let spec = specs::command("package digest").expect("package spec");
    assert_eq!(
        query_paths(spec),
        vec![
            PathBuf::from(
                "validation_artifacts/observability/command-roundtrip/package-digest-logs.json"
            ),
            PathBuf::from(
                "validation_artifacts/observability/command-roundtrip/package-digest-metrics.json"
            ),
            PathBuf::from(
                "validation_artifacts/observability/command-roundtrip/package-digest-traces.json"
            )
        ]
    );
    assert_eq!(
        roundtrip_path(spec, "explain").to_string_lossy(),
        "validation_artifacts/observability/command-roundtrip/package-digest-explain.json"
    );
    assert_eq!(status(&json!({"status": "pass"})), "pass");
    assert_eq!(status(&json!({})), "missing");
    assert_eq!(text(&json!({"run_id": "run"}), "run_id").unwrap(), "run");
    assert!(
        text(&json!({}), "run_id")
            .expect_err("missing run id")
            .contains("missing run_id")
    );
}

#[test]
fn query_and_explain_roundtrip_write_receipts_and_keep_candidate_binding() {
    let (root, candidate) = roundtrip_root("observe-roundtrip-query-explain");
    let spec = specs::command("package digest").expect("package spec");
    let logs = query_roundtrip(&root, spec, QueryKind::Logs, "run-fit", "corr-fit", 1)
        .expect("logs roundtrip receipt");
    assert_eq!(logs["candidate_digest"], candidate);
    assert_eq!(logs["query_kind"], "logs");
    assert!(root.join(roundtrip_path(spec, "logs")).is_file());

    let explain = explain_roundtrip(&root, spec, "run-fit", "corr-fit", 1)
        .expect("explain roundtrip receipt");
    assert_eq!(explain["status"], "pass");
    assert_eq!(explain["candidate_digest"], candidate);
    assert_eq!(explain["failure_class"], "none");
    assert!(root.join(roundtrip_path(spec, "explain-failure")).is_file());
    std::fs::remove_dir_all(root).expect("cleanup query explain");
}
