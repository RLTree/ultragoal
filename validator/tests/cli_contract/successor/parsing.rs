use super::successor::{
    CheckProfile, EffectClass, EvalAction, ExitClass, HelpTarget, MigrateAction, ObserveAction,
    OutputMode, PackageAction, ParseErrorId, ParseOutcome, SuccessorCommand, parse_args,
    render_help, version_text,
};

fn invocation(args: &[&str]) -> super::successor::ParsedInvocation {
    let ParseOutcome::Invocation(invocation) = parse_args(args.iter().copied()).unwrap() else {
        panic!("expected invocation");
    };
    invocation
}

#[test]
fn typed_commands_and_effects_cover_representative_routes() {
    let check = invocation(&["check", "routine"]);
    assert_eq!(
        check.command,
        SuccessorCommand::Check(CheckProfile::Routine)
    );
    assert_eq!(check.effect, EffectClass::WorkspaceWrite);

    let query = invocation(&["observe", "query", "--filter", "finding:open"]);
    assert_eq!(
        query.command,
        SuccessorCommand::Observe(ObserveAction::Query)
    );
    assert_eq!(query.effect, EffectClass::Read);

    let publish = invocation(&[
        "package",
        "publish",
        "--input",
        "dist/plugin.zip",
        "--provider",
        "marketplace",
        "--approve-publish",
    ]);
    assert_eq!(
        publish.command,
        SuccessorCommand::Package(PackageAction::Publish)
    );
    assert_eq!(publish.effect, EffectClass::ExternalWrite);

    let adapter = invocation(&[
        "eval",
        "adapter",
        "--spec",
        "eval/spec.json",
        "--provider",
        "openai",
    ]);
    assert_eq!(adapter.command, SuccessorCommand::Eval(EvalAction::Adapter));
    assert_eq!(adapter.effect, EffectClass::ExternalWrite);

    let retire = invocation(&[
        "migrate",
        "retire",
        "--plan",
        "migration/plan.json",
        "--approve-retirement",
    ]);
    assert_eq!(
        retire.command,
        SuccessorCommand::Migrate(MigrateAction::Retire)
    );
    assert_eq!(retire.effect, EffectClass::Destructive);
}

#[test]
fn human_and_json_modes_are_typed_and_singleton() {
    let parsed = invocation(&["--json", "inspect"]);
    assert_eq!(parsed.output_mode, OutputMode::Json);
    let parsed = invocation(&["inspect"]);
    assert_eq!(parsed.output_mode, OutputMode::Human);
    let error = parse_args(["--json", "inspect", "--json"]).unwrap_err();
    assert_eq!(error.error.id(), ParseErrorId::DuplicateOption);
    assert_eq!(error.output_mode, OutputMode::Json);
}

#[test]
fn root_group_and_command_help_are_catalog_backed() {
    let ParseOutcome::Help {
        target,
        output_mode,
    } = parse_args(["--json", "--help"]).unwrap()
    else {
        panic!("expected root help");
    };
    assert_eq!(target, HelpTarget::Root);
    assert_eq!(output_mode, OutputMode::Json);
    assert!(render_help(target, output_mode).contains("\"commands\":"));

    let ParseOutcome::Help { target, .. } = parse_args(["fit", "--help"]).unwrap() else {
        panic!("expected group help");
    };
    assert_eq!(target, HelpTarget::Group(super::successor::Group::Fit));
    let group_help = render_help(target, OutputMode::Human);
    assert!(group_help.contains("fit inspect"));
    assert!(group_help.contains("fit apply"));

    let ParseOutcome::Help { target, .. } = parse_args(["fit", "apply", "--help"]).unwrap() else {
        panic!("expected command help");
    };
    assert_eq!(
        target,
        HelpTarget::Command(SuccessorCommand::Fit(super::successor::FitAction::Apply))
    );
}

#[test]
fn version_and_exit_classes_are_stable_typed_data() {
    assert_eq!(
        parse_args(["--version"]),
        Ok(ParseOutcome::Version(OutputMode::Human))
    );
    let machine_version = version_text(OutputMode::Json);
    let version: serde_json::Value = serde_json::from_str(&machine_version).unwrap();
    assert_eq!(
        version["schema_version"],
        "harness-ultragoal.cli-version.v1"
    );
    assert_eq!(ExitClass::Success.code(), 0);
    assert_eq!(ExitClass::ActionableFinding.code(), 1);
    assert_eq!(ExitClass::InvalidInvocation.code(), 2);
    assert_eq!(ExitClass::BlockedAuthority.code(), 3);
    assert_eq!(ExitClass::UnsupportedCapability.code(), 4);
    assert_eq!(ExitClass::InternalFailure.code(), 70);
}

#[test]
fn required_options_and_option_values_fail_closed() {
    assert_eq!(
        parse_args(["prove", "--claim", "CL-SOURCE"])
            .unwrap_err()
            .error
            .id(),
        ParseErrorId::MissingRequiredOption
    );
    assert_eq!(
        parse_args(["prove", "--claim", "--help"])
            .unwrap_err()
            .error
            .id(),
        ParseErrorId::HelpValueConfusion
    );
    assert_eq!(
        parse_args(["observe", "export", "--approve-export=yes"])
            .unwrap_err()
            .error
            .id(),
        ParseErrorId::UnexpectedOptionValue
    );
}
