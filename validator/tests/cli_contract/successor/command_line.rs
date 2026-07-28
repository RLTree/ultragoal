use super::successor::command_contract::HelpTarget;
use super::successor::{
    LegacyCommand, OutputMode, ParseErrorId, ParseFailure, ParseOutcome, ParsedCommandLine,
    SuccessorCommand, WorkspaceRoot, parse_command_line,
};
use std::path::Path;

#[test]
fn typed_workspace_root_defaults_and_preserves_explicit_authority() {
    let parsed: ParsedCommandLine = parse_command_line(["inspect"]).expect("default root parses");
    let (root, _) = parsed.into_parts();
    let root: WorkspaceRoot = root;
    assert_eq!(root.into_path_buf(), Path::new("."));

    for args in [
        vec!["--root", "/private/typed-root", "inspect", "context"],
        vec!["inspect", "context", "--root", "/private/typed-root"],
        vec!["--json", "inspect", "--root", "/private/typed-root"],
    ] {
        let (root, _) = parse_command_line(args)
            .expect("explicit root parses")
            .into_parts();
        assert_eq!(root.into_path_buf(), Path::new("/private/typed-root"));
    }
}

#[test]
fn root_is_removed_before_help_and_compatibility_classification() {
    let (_, outcome) =
        parse_command_line(["--root", "/private/help-root", "fit", "apply", "--help"])
            .expect("rooted help")
            .into_parts();
    assert!(matches!(
        outcome,
        ParseOutcome::Help {
            target: HelpTarget::Command(SuccessorCommand::Fit(_)),
            output_mode: OutputMode::Human,
        }
    ));

    let (_, outcome) =
        parse_command_line(["archive", "--root", "/private/compatibility-root", "--json"])
            .expect("rooted compatibility intent")
            .into_parts();
    assert!(matches!(
        outcome,
        ParseOutcome::Compatibility {
            command: LegacyCommand::Archive,
            output_mode: OutputMode::Json,
        }
    ));
}

#[test]
fn malformed_root_after_help_is_outside_the_semantic_boundary() {
    let (root, outcome) = parse_command_line(["--root", "/private/help-root", "--help", "--root"])
        .expect("trailing root is ignored after help")
        .into_parts();
    assert_eq!(root.into_path_buf(), Path::new("/private/help-root"));
    assert!(matches!(
        outcome,
        ParseOutcome::Help {
            target: HelpTarget::Root,
            output_mode: OutputMode::Human,
        }
    ));
}

#[test]
fn malformed_roots_fail_with_typed_non_echoing_errors() {
    let cases: &[(Vec<&str>, ParseErrorId)] = &[
        (vec!["--json", "--root"], ParseErrorId::MissingOptionValue),
        (
            vec!["--root", "one", "--root", "two", "inspect"],
            ParseErrorId::DuplicateOption,
        ),
        (vec!["--root", " ", "inspect"], ParseErrorId::InvalidPath),
        (
            vec!["--root=private-inline", "inspect"],
            ParseErrorId::UnexpectedOptionValue,
        ),
    ];
    for (args, expected) in cases {
        let failure: ParseFailure =
            parse_command_line(args.iter().copied()).expect_err("root must fail");
        assert_eq!(failure.error.id(), *expected, "{args:?}");
        let rendered = failure.render();
        assert!(!rendered.contains("private-inline"));
        assert!(!rendered.contains("one"));
        assert!(!rendered.contains("two"));
    }
}

#[test]
fn root_counts_toward_the_fixed_command_line_bounds() {
    let oversized = "r".repeat(8193);
    let failure = parse_command_line(["--root".to_string(), oversized, "inspect".to_string()])
        .expect_err("oversized root must fail");
    assert_eq!(failure.error.id(), ParseErrorId::ArgumentTooLarge);
}
