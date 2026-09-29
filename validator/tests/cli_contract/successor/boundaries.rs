use super::successor::{MAX_ARGUMENT_BYTES, ParseErrorId, ParseOutcome, ParsedValue, parse_args};

const MAX_ARGUMENTS: usize = 256;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_PATH_BYTES: usize = 4096;

fn id(args: impl IntoIterator<Item = impl Into<std::ffi::OsString>>) -> ParseErrorId {
    parse_args(args).unwrap_err().error.id()
}

#[test]
fn argument_count_boundary_is_inclusive() {
    let at_limit = vec!["unknown"; MAX_ARGUMENTS];
    assert_ne!(id(at_limit), ParseErrorId::TooManyArguments);
    let over_limit = vec!["unknown"; MAX_ARGUMENTS + 1];
    assert_eq!(id(over_limit), ParseErrorId::TooManyArguments);
}

#[test]
fn per_argument_byte_boundary_is_inclusive() {
    let at_limit = "x".repeat(MAX_ARGUMENT_BYTES);
    assert_ne!(id([at_limit]), ParseErrorId::ArgumentTooLarge);
    let over_limit = "x".repeat(MAX_ARGUMENT_BYTES + 1);
    assert_eq!(id([over_limit]), ParseErrorId::ArgumentTooLarge);
}

#[test]
fn identifier_byte_boundary_is_inclusive() {
    let at_limit = "a".repeat(MAX_IDENTIFIER_BYTES);
    let ParseOutcome::Invocation(invocation) = parse_args([
        "prove".to_owned(),
        "--claim".to_owned(),
        at_limit.clone(),
        "--output".to_owned(),
        "proof/result.json".to_owned(),
    ])
    .unwrap() else {
        panic!("expected invocation");
    };
    assert!(invocation.arguments.iter().any(|argument| {
        matches!(&argument.value, ParsedValue::Identifier(value) if value == &at_limit)
    }));
    let over_limit = "a".repeat(MAX_IDENTIFIER_BYTES + 1);
    assert_eq!(
        id([
            "prove".to_owned(),
            "--claim".to_owned(),
            over_limit,
            "--output".to_owned(),
            "proof/result.json".to_owned(),
        ]),
        ParseErrorId::InvalidIdentifier
    );
}

#[test]
fn path_byte_boundary_is_inclusive_and_independent_from_argument_budget() {
    let at_limit = "p".repeat(MAX_PATH_BYTES);
    assert!(
        parse_args([
            "package".to_owned(),
            "build".to_owned(),
            "--output".to_owned(),
            at_limit,
        ])
        .is_ok()
    );
    let over_limit = "p".repeat(MAX_PATH_BYTES + 1);
    assert_eq!(
        id([
            "package".to_owned(),
            "build".to_owned(),
            "--output".to_owned(),
            over_limit,
        ]),
        ParseErrorId::InvalidPath
    );
}

#[test]
fn malformed_corpus_has_an_explicit_rejection_oracle() {
    let cases: &[(&[&str], ParseErrorId)] = &[
        (&[], ParseErrorId::EmptyInvocation),
        (&["unknown"], ParseErrorId::UnknownSubcommand),
        (&["fit"], ParseErrorId::MissingSubcommand),
        (&["next", "--mystery"], ParseErrorId::UnknownOption),
        (&["inspect", "write"], ParseErrorId::ImplicitWriteVerb),
        (
            &["inspect", "--effect=read"],
            ParseErrorId::EffectOverrideForbidden,
        ),
        (
            &["prove", "--claim", "--help"],
            ParseErrorId::HelpValueConfusion,
        ),
        (&["prove", "--claim"], ParseErrorId::MissingOptionValue),
        (
            &["prove", "--claim", "bad/value", "--output", "out"],
            ParseErrorId::InvalidIdentifier,
        ),
        (
            &["package", "build", "--output", "../escape"],
            ParseErrorId::InvalidPath,
        ),
        (
            &["observe", "export", "--approve-export=true"],
            ParseErrorId::UnexpectedOptionValue,
        ),
        (&["--version", "--help"], ParseErrorId::InvalidHelpPosition),
    ];
    for (args, expected) in cases {
        assert_eq!(id(args.iter().copied()), *expected, "case {args:?}");
    }
}
