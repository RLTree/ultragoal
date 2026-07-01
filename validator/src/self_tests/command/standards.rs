use serde_json::json;
use std::path::PathBuf;

fn args(root: PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}

#[test]
fn command_dispatch_routes_standards_gardener_rebind() {
    let root = crate::self_tests::boundaries::support::temp_root("standards-command-dispatch");
    crate::json_boundary::write_json(&root.join("plugin-manifest-draft.json"), &json!({}))
        .expect("manifest");
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "standards-gardener",
            "rebind",
            "--receipt",
            "target/not-standards.json",
        ],
    ))
    .expect("standards command fail-closes with receipt");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/standards-gardener-rebind.json"),
    )
    .expect("observability receipt");
    assert_eq!(receipt["status"], "fail");
    assert!(
        receipt["why_failed"]
            .as_str()
            .expect("why")
            .contains("under validation_artifacts/standards-gardener")
    );
    std::fs::remove_dir_all(root).expect("cleanup standards command dispatch");
}

#[test]
fn command_parser_rejects_incomplete_standards_gardener_command_without_fallback_authority() {
    let error = crate::parse_command(&["standards-gardener".to_string()]).expect_err("usage error");
    assert!(error.contains("usage:"), "{error}");
}
