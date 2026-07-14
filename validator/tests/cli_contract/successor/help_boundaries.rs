use super::successor::command_contract::HelpTarget;
use super::successor::{
    FitAction, Group, OutputMode, ParseErrorId, ParseOutcome, SuccessorCommand, parse_args,
    render_help,
};

fn expected_help(target: HelpTarget, output_mode: OutputMode) -> ParseOutcome {
    ParseOutcome::Help {
        target,
        output_mode,
    }
}

#[test]
fn root_and_group_help_stop_before_trailing_route_tokens() {
    assert_eq!(
        parse_args(["--help", "fit", "apply"]),
        Ok(expected_help(HelpTarget::Root, OutputMode::Human))
    );
    assert_eq!(
        parse_args(["-h", "fit", "apply"]),
        Ok(expected_help(HelpTarget::Root, OutputMode::Human))
    );
    assert_eq!(
        parse_args(["fit", "--help", "apply"]),
        Ok(expected_help(
            HelpTarget::Group(Group::Fit),
            OutputMode::Human,
        ))
    );
    assert_eq!(
        parse_args(["fit", "-h", "apply"]),
        Ok(expected_help(
            HelpTarget::Group(Group::Fit),
            OutputMode::Human,
        ))
    );
}

#[test]
fn command_help_and_both_global_json_positions_are_exact() {
    let command = HelpTarget::Command(SuccessorCommand::Fit(FitAction::Apply));
    assert_eq!(
        parse_args(["fit", "apply", "--help"]),
        Ok(expected_help(command, OutputMode::Human))
    );
    assert_eq!(
        parse_args(["--json", "fit", "apply", "--help"]),
        Ok(expected_help(command, OutputMode::Json))
    );
    assert_eq!(
        parse_args(["fit", "apply", "--json", "--help"]),
        Ok(expected_help(command, OutputMode::Json))
    );
    assert_eq!(
        parse_args(["fit", "--json", "apply", "--help"]),
        Ok(expected_help(command, OutputMode::Json))
    );
}

#[test]
fn trailing_canary_cannot_change_help_target_mode_or_rendered_output() {
    let canary = "never-echo-after-help-canary-7019";
    let root = parse_args(["--help", "fit", "apply", "--json", canary]).unwrap();
    assert_eq!(root, expected_help(HelpTarget::Root, OutputMode::Human));
    let ParseOutcome::Help {
        target,
        output_mode,
    } = root
    else {
        panic!("expected help");
    };
    assert!(!render_help(target, output_mode).contains(canary));

    let group = parse_args(["fit", "--help", "apply", "--json", canary]).unwrap();
    assert_eq!(
        group,
        expected_help(HelpTarget::Group(Group::Fit), OutputMode::Human)
    );
    let command = parse_args(["--json", "fit", "apply", "--help", canary]).unwrap();
    assert_eq!(
        command,
        expected_help(
            HelpTarget::Command(SuccessorCommand::Fit(FitAction::Apply)),
            OutputMode::Json,
        )
    );
    let ParseOutcome::Help {
        target,
        output_mode,
    } = command
    else {
        panic!("expected help");
    };
    assert!(!render_help(target, output_mode).contains(canary));
}

#[test]
fn malformed_tokens_fail_before_help_but_are_ignored_after_the_stop_boundary() {
    assert_eq!(
        parse_args(["fit", "bogus", "--help"])
            .unwrap_err()
            .error
            .id(),
        ParseErrorId::UnknownSubcommand
    );
    assert_eq!(
        parse_args(["fit", "apply", "--plan", "--help"])
            .unwrap_err()
            .error
            .id(),
        ParseErrorId::HelpValueConfusion
    );
    assert_eq!(
        parse_args(["fit", "apply", "--help", "--plan"]),
        Ok(expected_help(
            HelpTarget::Command(SuccessorCommand::Fit(FitAction::Apply)),
            OutputMode::Human,
        ))
    );
    assert_eq!(
        parse_args(["fit", "--help", "--version", "--target"]),
        Ok(expected_help(
            HelpTarget::Group(Group::Fit),
            OutputMode::Human,
        ))
    );
    assert_eq!(
        parse_args(["--help", "--version", "--json"]),
        Ok(expected_help(HelpTarget::Root, OutputMode::Human))
    );
}
