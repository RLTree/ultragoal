use super::successor::clap_grammar::parser_command;
use super::successor::command_contract::{CommandDescriptor, HelpTarget};
use super::successor::{
    EffectClass, Group, OptionName, OptionSpec, OutputMode, ParseErrorId, ParseOutcome,
    ParsedValue, ValueKind, catalog, effect_name, parse_args, render_help,
};
use clap::{ArgAction, builder::ValueRange};
use std::collections::BTreeSet;
#[test]
fn catalog_exposes_exactly_the_ten_contract_groups() {
    let groups: BTreeSet<_> = catalog()
        .iter()
        .map(|descriptor| descriptor.command.group())
        .collect();
    assert_eq!(groups, BTreeSet::from(Group::ALL));
    assert_eq!(Group::ALL.len(), 10);
}
#[test]
fn every_catalog_route_is_unique_and_parses_from_its_descriptor() {
    let mut routes = BTreeSet::new();
    for descriptor in catalog() {
        let options: &[OptionSpec] = descriptor.options;
        assert!(routes.insert((descriptor.command.group(), descriptor.subcommand)));
        let mut args = vec![descriptor.command.group().as_str().to_owned()];
        if let Some(subcommand) = descriptor.subcommand {
            args.push(subcommand.to_owned());
        }
        for option in options.iter().filter(|option| option.required) {
            args.push(option.name.as_str().to_owned());
            match option.kind {
                ValueKind::Flag => {}
                ValueKind::Identifier => args.push("candidate-1".to_owned()),
                ValueKind::RelativePath => args.push("artifacts/result.json".to_owned()),
                ValueKind::HostPath => args.push("/package".to_owned()),
            }
        }
        let ParseOutcome::Invocation(parsed) = parse_args(args).expect("catalog route parses")
        else {
            panic!("catalog route did not produce an invocation");
        };
        assert_eq!(parsed.command, descriptor.command);
        assert_eq!(parsed.effect, descriptor.effect);
        assert_eq!(
            parsed.arguments.len(),
            descriptor
                .options
                .iter()
                .filter(|option| option.required)
                .count()
        );
        assert!(
            parsed
                .arguments
                .iter()
                .all(|argument| match &argument.value {
                    ParsedValue::Flag => true,
                    ParsedValue::Identifier(value) => !value.is_empty(),
                    ParsedValue::RelativePath(path) => !path.as_str().is_empty(),
                    ParsedValue::HostPath(path) => {
                        path.is_valid() && path.as_path().is_absolute()
                    }
                })
        );
    }
}
#[test]
fn compiled_clap_grammar_covers_every_catalog_route_and_option() {
    let command = parser_command();
    command.clone().debug_assert();
    let clap_groups: BTreeSet<_> = command
        .get_subcommands()
        .map(|subcommand| subcommand.get_name())
        .collect();
    let expected_groups: BTreeSet<_> = Group::ALL.iter().map(|group| group.as_str()).collect();
    assert_eq!(clap_groups, expected_groups);
    for descriptor in catalog() {
        let group = command
            .find_subcommand(descriptor.command.group().as_str())
            .expect("compiled clap group");
        let active = descriptor.subcommand.map_or(group, |subcommand| {
            group
                .find_subcommand(subcommand)
                .expect("compiled clap subcommand")
        });
        let clap_options: BTreeSet<_> = active
            .get_arguments()
            .filter_map(|argument| argument.get_long())
            .filter(|name| !matches!(*name, "json" | "help" | "version"))
            .collect();
        let expected_options: BTreeSet<_> = descriptor
            .options
            .iter()
            .map(|option| option.name.as_str().trim_start_matches("--"))
            .collect();
        assert_eq!(clap_options, expected_options);
        for option in descriptor.options {
            let name = option.name.as_str().trim_start_matches("--");
            let argument = active
                .get_arguments()
                .find(|argument| argument.get_long() == Some(name))
                .expect("compiled clap option");
            assert_eq!(argument.is_required_set(), option.required);
            assert!(matches!(argument.get_action(), &ArgAction::Set));
            let expected_arity = match option.kind {
                ValueKind::Flag => ValueRange::EMPTY,
                ValueKind::Identifier | ValueKind::RelativePath | ValueKind::HostPath => {
                    ValueRange::SINGLE
                }
            };
            assert_eq!(argument.get_num_args(), Some(expected_arity));
            let value_name = argument
                .get_value_names()
                .and_then(|names| names.first())
                .map(|name| name.as_str());
            assert_eq!(
                value_name,
                match option.kind {
                    ValueKind::Flag => None,
                    ValueKind::Identifier => Some("ID"),
                    ValueKind::RelativePath => Some("RELATIVE_PATH"),
                    ValueKind::HostPath => Some("HOST_PATH"),
                }
            );
        }
    }
}
#[test]
fn clap_requiredness_arity_and_singleton_behavior_match_every_option_spec() {
    for descriptor in catalog() {
        for option in descriptor.options {
            let omitted = required_args(descriptor, Some(option.name));
            let omission = parse_args(omitted.clone());
            if option.required {
                assert_eq!(
                    omission.unwrap_err().error.id(),
                    ParseErrorId::MissingRequiredOption
                );
            } else {
                assert!(omission.is_ok());
            }
            let mut once = omitted.clone();
            push_option(&mut once, option.name, option.kind);
            assert!(parse_args(once).is_ok());
            let mut duplicate = omitted.clone();
            push_option(&mut duplicate, option.name, option.kind);
            push_option(&mut duplicate, option.name, option.kind);
            assert_eq!(
                parse_args(duplicate).unwrap_err().error.id(),
                ParseErrorId::DuplicateOption
            );

            let mut wrong_arity = omitted;
            if option.kind == ValueKind::Flag {
                wrong_arity.push(format!("{}=true", option.name.as_str()));
                assert_eq!(
                    parse_args(wrong_arity).unwrap_err().error.id(),
                    ParseErrorId::UnexpectedOptionValue
                );
            } else {
                wrong_arity.push(option.name.as_str().to_owned());
                assert_eq!(
                    parse_args(wrong_arity).unwrap_err().error.id(),
                    ParseErrorId::MissingOptionValue
                );
            }
        }
    }
}
#[test]
fn help_is_a_projection_of_the_same_compiled_catalog() {
    let human = render_help(HelpTarget::Root, OutputMode::Human);
    let json = render_help(HelpTarget::Root, OutputMode::Json);
    assert_eq!(
        human,
        include_str!("goldens/root-help.txt"),
        "human help snapshot drift"
    );
    assert_eq!(
        json,
        include_str!("goldens/root-help.json").trim_end(),
        "machine help snapshot drift"
    );
    assert!(human.contains("Usage: ultragoal [--json]"));
    assert!(!human.contains("--format"));
    assert!(!json.contains("--format"));
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid machine help JSON");
    assert!(json.starts_with("{\"schema_version\":\"harness-ultragoal.cli-help.v1\""));
    assert_eq!(
        parsed["commands"].as_array().map(Vec::len),
        Some(catalog().len())
    );
    for descriptor in catalog() {
        let route = match descriptor.subcommand {
            Some(subcommand) => format!("{} {subcommand}", descriptor.command.group().as_str()),
            None => descriptor.command.group().as_str().to_owned(),
        };
        assert!(human.contains(&route), "missing human route {route}");
        assert!(json.contains(descriptor.purpose));
        assert!(json.contains(effect_name(descriptor.effect)));
    }
}
#[test]
fn sensitive_effects_are_never_attached_to_read_routes() {
    for descriptor in catalog() {
        match (descriptor.command.group(), descriptor.subcommand) {
            (Group::Fit, Some("apply"))
            | (Group::Check, Some("routine"))
            | (Group::Prove, _)
            | (Group::Package, Some("inventory" | "build" | "install-test"))
            | (Group::Eval, Some("run" | "harvest" | "promote"))
            | (Group::Migrate, Some("apply")) => {
                assert_eq!(descriptor.effect, EffectClass::WorkspaceWrite)
            }
            (Group::Observe, Some("export"))
            | (Group::Package, Some("publish"))
            | (Group::Eval, Some("adapter")) => {
                assert_eq!(descriptor.effect, EffectClass::ExternalWrite)
            }
            (Group::Migrate, Some("retire")) => {
                assert_eq!(descriptor.effect, EffectClass::Destructive)
            }
            _ => assert_eq!(descriptor.effect, EffectClass::Read),
        }
    }
}

fn required_args(descriptor: &CommandDescriptor, omit: Option<OptionName>) -> Vec<String> {
    let mut args = vec![descriptor.command.group().as_str().to_owned()];
    if let Some(subcommand) = descriptor.subcommand {
        args.push(subcommand.to_owned());
    }
    for option in descriptor
        .options
        .iter()
        .filter(|option| option.required && Some(option.name) != omit)
    {
        push_option(&mut args, option.name, option.kind);
    }
    args
}

fn push_option(args: &mut Vec<String>, name: OptionName, kind: ValueKind) {
    args.push(name.as_str().to_owned());
    match kind {
        ValueKind::Flag => {}
        ValueKind::Identifier => args.push("candidate-1".to_owned()),
        ValueKind::RelativePath => args.push("artifacts/result.json".to_owned()),
        ValueKind::HostPath => args.push("/package".to_owned()),
    }
}
