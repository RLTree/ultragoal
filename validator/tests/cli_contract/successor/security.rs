use super::successor::{EffectClass, ParseErrorId, ParseOutcome, parse_args};
use std::ffi::OsString;

fn error_id(args: impl IntoIterator<Item = impl Into<OsString>>) -> ParseErrorId {
    parse_args(args).unwrap_err().error.id()
}

#[test]
fn path_escape_and_ambiguous_path_encodings_are_rejected() {
    for path in [
        "/absolute",
        "../escape",
        "safe/../escape",
        "safe//file",
        "safe/./file",
        "C:drive",
        "safe\\file",
        "%2e%2e/escape",
        "~/escape",
        "safe\0escape",
    ] {
        assert_eq!(
            error_id(["prove", "--claim", "CL-SOURCE", "--output", path]),
            ParseErrorId::InvalidPath,
            "path should fail closed"
        );
    }
}

#[test]
fn current_repository_marker_is_accepted_only_for_repository_targets() {
    assert!(parse_args(["fit", "inspect", "--target", "."]).is_ok());
    assert!(parse_args(["check", "routine", "--target", "."]).is_ok());
    assert_eq!(
        error_id(["prove", "--claim", "CL-SOURCE", "--output", "."]),
        ParseErrorId::InvalidPath
    );
    assert_eq!(
        error_id(["migrate", "apply", "--plan", ".", "--accept-plan", "plan-1"]),
        ParseErrorId::InvalidPath
    );
}

#[test]
fn effect_override_and_implicit_write_attempts_are_rejected() {
    for args in [
        vec!["fit", "apply", "--effect=read"],
        vec!["check", "routine", "--dry-run"],
        vec!["migrate", "retire", "--read-only"],
        vec!["inspect", "--allow-write"],
    ] {
        assert_eq!(error_id(args), ParseErrorId::EffectOverrideForbidden);
    }
    for args in [
        vec!["inspect", "write"],
        vec!["fit", "save"],
        vec!["delete"],
        vec!["next", "publish"],
    ] {
        assert_eq!(error_id(args), ParseErrorId::ImplicitWriteVerb);
    }
}

#[test]
fn unknown_input_never_appears_in_human_or_machine_errors() {
    let canary = "never-echo-canary-94731";
    for args in [
        vec![canary],
        vec!["inspect", canary],
        vec!["inspect", "--json", canary],
        vec!["prove", "--claim", canary, "--output", "../escape"],
    ] {
        let failure = parse_args(args).unwrap_err();
        assert!(!failure.error.to_string().contains(canary));
        assert!(!failure.render().contains(canary));
    }
}

#[test]
fn duplicate_singletons_unknown_options_and_extra_positionals_fail() {
    assert_eq!(
        error_id(["diagnose", "--finding", "F-1", "--finding", "F-2"]),
        ParseErrorId::DuplicateOption
    );
    assert_eq!(error_id(["next", "--mystery"]), ParseErrorId::UnknownOption);
    assert_eq!(
        error_id([
            "observe",
            "export",
            "--output",
            "out/events.json",
            "--approve-export",
            "--approve-export",
        ]),
        ParseErrorId::DuplicateOption
    );
    assert_eq!(
        error_id(["diagnose", "extra"]),
        ParseErrorId::UnknownSubcommand
    );
}

#[test]
fn machine_errors_are_valid_versioned_json_without_ansi_or_input_echo() {
    let canary = "never-render-machine-canary-581";
    let failure = parse_args(["--json", canary]).unwrap_err();
    let rendered = failure.render();
    let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(parsed["schema_version"], "harness-ultragoal.cli-error.v1");
    assert_eq!(parsed["exit_code"], 2);
    assert!(!rendered.contains(canary));
    assert!(!rendered.contains("\u{1b}["));
}

#[test]
fn attached_help_value_confusion_is_rejected_before_help_rendering() {
    assert_eq!(
        error_id(["prove", "--claim=--help", "--output", "out/proof.json"]),
        ParseErrorId::HelpValueConfusion
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_input_fails_without_lossy_echo() {
    use std::os::unix::ffi::OsStringExt;
    let input = OsString::from_vec(vec![0xff, 0xfe, b'X']);
    let failure = parse_args([input]).unwrap_err();
    assert_eq!(failure.error.id(), ParseErrorId::NonUtf8Argument);
    assert_eq!(failure.error.exit_class().code(), 2);
}

#[test]
fn parse_results_are_deterministic_and_do_not_downgrade_effects() {
    let args = [
        "fit",
        "apply",
        "--plan",
        "/tmp/fit-plan.json",
        "--accept-plan",
        "sha256:1234",
    ];
    let first = parse_args(args).unwrap();
    let second = parse_args(args).unwrap();
    assert_eq!(first, second);
    let ParseOutcome::Invocation(invocation) = first else {
        panic!("expected invocation");
    };
    assert_eq!(invocation.effect, EffectClass::WorkspaceWrite);
}

#[test]
fn bounded_mixed_input_matrix_never_panics_or_changes_identity() {
    const TOKENS: &[&str] = &[
        "inspect",
        "next",
        "fit",
        "apply",
        "observe",
        "export",
        "migrate",
        "retire",
        "--json",
        "--help",
        "--claim",
        "--output",
        "--effect=read",
        "../escape",
        "unknown-canary-628",
        "",
    ];
    let mut state = 0x9e37_79b9_u32;
    for _ in 0..1024 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let length = (state as usize % 6) + 1;
        let mut args = Vec::with_capacity(length);
        for offset in 0..length {
            let index =
                state.wrapping_add((offset as u32).wrapping_mul(17)) as usize % TOKENS.len();
            args.push(TOKENS[index]);
        }
        let first = parse_args(args.iter().copied());
        let second = parse_args(args.iter().copied());
        assert_eq!(first, second);
        // This is a mixed deterministic fuzz matrix: valid combinations may
        // succeed; malformed-corpus rejection identities are asserted separately.
        if let Err(failure) = first {
            let rendered = failure.render();
            assert!(!rendered.contains("\u{1b}["));
        }
    }
}
