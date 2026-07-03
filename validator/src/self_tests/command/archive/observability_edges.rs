use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn args(root: PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}

fn package_root(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/file.txt"), "package payload\n").expect("file");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["docs/file.txt"]}),
    );
    root
}

fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or("")
}

#[test]
fn archive_parse_defaults_and_rejects_package_escaping_observability_receipts() {
    let command = crate::parse_command(&[
        "archive".to_string(),
        "build".to_string(),
        "--zip".to_string(),
        "receipts/candidate.zip".to_string(),
        "--receipt".to_string(),
        "receipts/archive.json".to_string(),
    ])
    .expect("parse archive command");

    let rendered = format!("{command:?}");
    assert!(rendered.contains("validation_artifacts/observability/archive-build.json"));
    assert!(rendered.contains("harness-ultragoal-plugin-proposal"));
    assert!(rendered.contains("candidate_review_anchor"));

    let error = crate::parse_command(&[
        "archive".to_string(),
        "build".to_string(),
        "--zip".to_string(),
        "receipts/candidate.zip".to_string(),
        "--receipt".to_string(),
        "receipts/archive.json".to_string(),
        "--observability-receipt".to_string(),
        "../outside.json".to_string(),
    ])
    .expect_err("unsafe observability receipt rejected");
    assert!(error.contains("archive observability receipt"), "{error}");
}

#[test]
fn archive_command_records_missing_manifest_as_fail_closed_observability() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "archive-observability-no-manifest",
    );
    std::fs::create_dir_all(&root).expect("root");
    let observability = "observability/archive-build-missing-manifest.json";

    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "archive",
            "build",
            "--zip",
            "receipts/candidate.zip",
            "--receipt",
            "receipts/archive.json",
            "--observability-receipt",
            observability,
            "--zip-root",
            "harness-ultragoal",
        ],
    ))
    .expect("archive command emits fail-closed observability");

    assert_eq!(code, 1);
    let obs = crate::json_boundary::read_json(&root.join(observability)).expect("observability");
    assert_eq!(text(&obs, "status"), "fail");
    assert_eq!(text(&obs, "candidate_digest"), crate::digest::ZERO);
    assert_eq!(text(&obs, "failure_class"), "archive_build_failure");
    assert!(
        text(&obs, "why_failed").contains("json read failed")
            && text(&obs, "why_failed").contains("metadata failed"),
        "{obs}"
    );
    assert!(!text(&obs, "why_failed").contains(&root.to_string_lossy().to_string()));
    std::fs::remove_dir_all(root).expect("cleanup missing manifest");
}

#[test]
fn archive_command_propagates_observability_receipt_write_errors() {
    let root = package_root("archive-observability-receipt-write-error");
    std::fs::write(root.join("observability"), b"not a directory")
        .expect("block observability dir");
    let error = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "archive",
            "build",
            "--zip",
            "receipts/candidate.zip",
            "--receipt",
            "receipts/archive.json",
            "--observability-receipt",
            "observability/archive-build.json",
            "--zip-root",
            "harness-ultragoal",
        ],
    ))
    .expect_err("observability receipt write failure propagates");

    assert!(error.contains("create parent failed"), "{error}");
    assert!(
        !error.contains(&root.to_string_lossy().to_string()),
        "{error}"
    );
    std::fs::remove_dir_all(root).expect("cleanup observability write error");
}
