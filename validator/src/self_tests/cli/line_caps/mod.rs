use serde_json::{Value, json};
use std::path::{Path, PathBuf};

mod edges;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_file(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, body).expect("write file");
}

fn package_root(label: &str, files: &[(&str, String)]) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    let resources = files
        .iter()
        .map(|(rel, body)| {
            write_file(&root.join(rel), body);
            json!(rel)
        })
        .collect::<Vec<_>>();
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources": resources}),
    );
    root
}

fn args(root: PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}

#[test]
fn line_caps_parser_routes_to_dedicated_command() {
    let raw = ["line-caps", "check", "--strict", "--jobs", "2"];
    let command =
        crate::cli::line_caps::parse(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("line caps parse")
            .expect("line caps command");
    assert_eq!(command.jobs, Some(2));
    assert_eq!(
        command.receipt,
        PathBuf::from("validation_artifacts/observability/line-cap-check.json")
    );
    let missing = ["line-caps", "check"];
    let err = crate::parse_command(&missing.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .expect_err("strict flag required");
    assert!(err.contains("requires --strict"), "{err}");
    let err = crate::parse_command(
        &["line-caps", "check", "--strict", "--jobs"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("missing jobs value");
    assert!(err.contains("missing value for --jobs"), "{err}");
    let err = crate::parse_command(
        &["line-caps", "check", "--strict", "--bogus"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("unknown argument");
    assert!(err.contains("unknown line-caps check argument"), "{err}");
}

#[test]
fn line_caps_command_writes_pass_observability_receipt() {
    let root = package_root(
        "line-caps-pass",
        &[("validator/src/lib.rs", "pub fn ok() {}\n".to_string())],
    );
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &["line-caps", "check", "--strict", "--jobs", "2"],
    ))
    .expect("line caps pass");
    assert_eq!(code, 0);
    let receipt_path = root.join("validation_artifacts/observability/line-cap-check.json");
    let receipt = crate::json_boundary::read_json(&receipt_path).expect("receipt");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    assert_eq!(
        receipt["schema"],
        crate::cli::observe::types::RECEIPT_SCHEMA
    );
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["candidate_digest"], candidate);
    assert_eq!(receipt["check_id"], "line-caps-check-observability-binding");
    assert_eq!(receipt["claim_id"], "line_cap_check");
    assert_eq!(receipt["event"]["operation"], "line-caps.check");
    assert_eq!(receipt["event"]["failure_class"], "none");
    assert!(receipt["event"]["duration_ms"].as_u64().unwrap() > 0);
    assert_eq!(receipt["event"]["worker_count"], 1);
    assert_eq!(receipt["event"]["task_count"], 1);
    assert_eq!(receipt["event"]["queue_depth"], 1);
    assert!(
        receipt["trace"]["child_spans"]
            .as_array()
            .expect("child spans")
            .iter()
            .all(|span| span["parent_span_id"] == receipt["trace"]["span_id"])
    );
    assert!(
        receipt["supported_claims"]
            .as_array()
            .expect("supported")
            .iter()
            .any(|item| item.as_str() == Some("line_cap_check"))
    );
    let absolute_receipt = root.join("target/absolute-line-cap-check.json");
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "line-caps",
            "check",
            "--strict",
            "--receipt",
            absolute_receipt.to_str().expect("utf8 path"),
        ],
    ))
    .expect("absolute line caps receipt");
    assert_eq!(code, 0);
    assert!(absolute_receipt.is_file());
    std::fs::remove_dir_all(root).expect("cleanup line caps pass");
}

#[test]
fn line_caps_command_fails_over_cap_with_repair_fields() {
    let body = (0..=crate::audit::plugin::laws::MAX_SOURCE_LINES)
        .map(|index| format!("// line {index}\n"))
        .collect::<String>();
    let root = package_root("line-caps-fail", &[("validator/src/too_large.rs", body)]);
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &["line-caps", "check", "--strict"],
    ))
    .expect("line caps fail receipt");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/line-cap-check.json"),
    )
    .expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["event"]["failure_class"], "line_cap_failure");
    assert_eq!(receipt["where_failed"], "line-caps.check");
    assert!(
        receipt["why_failed"]
            .as_str()
            .expect("why")
            .contains("plugin_self_law_line_cap_exceeded:validator/src/too_large.rs:251")
    );
    assert!(
        receipt["next_repair"]
            .as_str()
            .expect("next repair")
            .contains("split the named file")
    );
    assert!(
        receipt["supported_claims"]
            .as_array()
            .expect("supported")
            .is_empty()
    );
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|item| item.as_str() == Some("update_goal_eligibility"))
    );
    std::fs::remove_dir_all(root).expect("cleanup line caps fail");
}

#[test]
fn line_caps_command_fails_when_no_source_paths_exist() {
    let root = package_root("line-caps-no-source", &[]);
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &["line-caps", "check", "--strict"],
    ))
    .expect("line caps no source receipt");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/line-cap-check.json"),
    )
    .expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert!(
        receipt["why_failed"]
            .as_str()
            .expect("why")
            .contains("line_cap_no_source_paths")
    );
    std::fs::remove_dir_all(root).expect("cleanup no source");
}

#[test]
fn line_caps_command_reports_candidate_digest_errors_before_receipt_claim() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("line-caps-no-manifest");
    std::fs::create_dir_all(&root).expect("temp root");
    let err = crate::cli::line_caps::run(
        &root,
        &crate::cli::line_caps::LineCapsCommand {
            receipt: PathBuf::from("validation_artifacts/observability/line-cap-check.json"),
            jobs: Some(1),
        },
    )
    .expect_err("missing manifest blocks candidate receipt");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup no manifest");
}
