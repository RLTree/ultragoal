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
    let root = crate::self_tests::boundaries::support::repo_root();
    let err = crate::command_run::run_with_exit_code(args(
        root,
        &[
            "standards-gardener",
            "rebind",
            "--receipt",
            "target/not-standards.json",
        ],
    ))
    .expect_err("standards command rejects non-governed receipt path");
    assert!(err.contains("under validation_artifacts/standards-gardener"));
}

#[test]
fn command_parser_rejects_incomplete_standards_gardener_command_without_fallback_authority() {
    let error = crate::parse_command(&["standards-gardener".to_string()]).expect_err("usage error");
    assert!(error.contains("usage:"), "{error}");
}
