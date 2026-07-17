use super::{cli_option_role, parse_command, parse_public_args_from};
use crate::cli::successor::{LegacyCommand, ParseOutcome};
use crate::command::Command;

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_string()).collect()
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

#[test]
fn accepted_successor_reads_own_the_public_route() {
    for args in [
        &["next"][..],
        &["--json", "inspect", "context"][..],
        &["--json", "inspect", "inventory"][..],
        &["diagnose"][..],
    ] {
        assert!(matches!(
            parse_command(&strings(args)).unwrap(),
            Command::Successor(_)
        ));
    }
}

#[test]
fn retained_legacy_routes_are_not_silently_reclassified() {
    assert!(matches!(
        parse_command(&strings(&["help"])).unwrap(),
        Command::Help
    ));
    assert!(matches!(
        parse_command(&strings(&["package", "inventory"])).unwrap(),
        Command::PackageInventory(_)
    ));
    assert!(matches!(
        parse_command(&strings(&["package", "digest"])).unwrap(),
        Command::PackageDigest
    ));
}

#[test]
fn production_entry_routes_known_legacy_intent_to_non_effectful_guidance() {
    for (args, expected) in [
        (&["--json", "help"][..], LegacyCommand::Help),
        (
            &["--json", "current-state", "--help"][..],
            LegacyCommand::Help,
        ),
        (
            &["--json", "observe", "logs", "query", "--help"][..],
            LegacyCommand::Help,
        ),
        (
            &["--json", "package", "digest", "--help"][..],
            LegacyCommand::Help,
        ),
    ] {
        let parsed = parse_public_args_from(strings(args)).expect("legacy guidance parses");
        assert!(matches!(
            parsed.command,
            Command::Successor(ParseOutcome::Compatibility { command, .. }) if command == expected
        ));
    }
    let error = parse_public_args_from(strings(&["--json", "NEVER_ECHO_CANARY_8841"]))
        .expect_err("unknown input remains a typed failure");
    assert!(error.contains("harness-ultragoal.cli-error.v1"));
    assert!(!error.contains("NEVER_ECHO_CANARY_8841"));
}

#[test]
fn public_root_option_failures_use_the_typed_machine_contract() {
    for args in [
        &["--json", "--root"][..],
        &["--json", "--root", "one", "--root", "two", "inspect"][..],
    ] {
        let error = parse_public_args_from(strings(args))
            .err()
            .expect("invalid root option must fail");
        assert!(error.contains("harness-ultragoal.cli-error.v1"));
        assert!(!error.contains("one"));
        assert!(!error.contains("two"));
    }
}

#[test]
fn malformed_successor_route_keeps_the_typed_machine_error() {
    let error = parse_command(&strings(&["--json", "inspect", "unknown"])).unwrap_err();
    assert!(error.contains("harness-ultragoal.cli-error.v1"));
    assert!(error.contains("CLI_UNKNOWN_SUBCOMMAND"));
}
