use super::{cli_option_role, parse_command};
use crate::command::Command;
use std::path::PathBuf;

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_string()).collect()
}

#[test]
fn source_audit_parser_preserves_target_repo_authority_path() {
    let command = parse_command(&strings(&[
        "source",
        "audit",
        "--receipt",
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "--target-repo",
        "target-plugin-repo",
        "--mode",
        "strict_fixtures",
        "--jobs",
        "8",
    ]))
    .expect("source audit command parses");

    let Command::Audit {
        target_repo,
        mode,
        jobs,
        ..
    } = command
    else {
        panic!("expected source audit command");
    };
    assert_eq!(target_repo, Some(PathBuf::from("target-plugin-repo")));
    assert_eq!(mode, "strict_fixtures");
    assert_eq!(jobs, Some(8));
}

#[test]
fn archive_zip_options_have_product_role_names() {
    assert_eq!(cli_option_role("--zip"), "archive zip path");
    assert_eq!(cli_option_role("--zip-root"), "archive zip root");
}

#[test]
#[should_panic(expected = "unregistered CLI option product role")]
fn unknown_cli_option_roles_fail_closed() {
    let _ = cli_option_role("--new-unregistered-path");
}
