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
    let root = crate::self_tests::boundaries::support::temp_root(label);
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
fn review_target_command_emits_pass_observability() {
    let root = package_root("review-target-observability-pass");
    let receipt = root
        .join("receipts/review-target.json")
        .to_string_lossy()
        .to_string();
    let observability = "observability/review-target-build.json";
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "review-target",
            "build",
            "--receipt",
            &receipt,
            "--observability-receipt",
            observability,
        ],
    ))
    .expect("review target command");
    assert_eq!(code, 0);
    let obs = crate::json_boundary::read_json(&root.join(observability)).expect("observability");
    assert_eq!(text(&obs, "status"), "pass");
    assert_eq!(text(&obs, "operation"), "review-target.build");
    assert_eq!(
        text(&obs, "check_id"),
        "review-target-build-observability-binding"
    );
    assert_eq!(
        text(&obs, "claim_id"),
        "review_target_source_local_observability"
    );
    assert_eq!(obs["event"]["task_count"], 2);
    std::fs::remove_dir_all(root).expect("cleanup review target pass");
}

#[test]
fn review_target_command_emits_fail_closed_observability() {
    let root = package_root("review-target-observability-fail");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["../escape.txt"]}),
    );
    let observability = "observability/review-target-build-fail.json";
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "review-target",
            "build",
            "--receipt",
            "receipts/review-target.json",
            "--observability-receipt",
            observability,
        ],
    ))
    .expect("review target command returns fail code");
    assert_eq!(code, 1);
    let obs = crate::json_boundary::read_json(&root.join(observability)).expect("observability");
    assert_eq!(text(&obs, "status"), "fail");
    assert_eq!(text(&obs, "failure_class"), "review_target_build_failure");
    assert!(text(&obs, "why_failed").contains("review target is not closed"));
    assert!(
        obs["blocked_claims"]
            .as_array()
            .expect("blocked claims")
            .iter()
            .any(|claim| claim == "update_goal_eligibility")
    );
    std::fs::remove_dir_all(root).expect("cleanup review target fail");
}

#[test]
fn review_target_observability_receipt_path_validation_is_fail_closed() {
    for raw in [
        vec!["--observability-receipt", ""],
        vec!["--observability-receipt", "../outside.json"],
    ] {
        let args = raw.iter().map(|item| item.to_string()).collect::<Vec<_>>();
        let error = crate::cli::review::target::observability_receipt(&args)
            .expect_err("unsafe observability receipt rejected");
        assert!(
            error.contains("review target observability receipt"),
            "{error}"
        );
    }
}
