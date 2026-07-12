use super::catalog::catalog;
use super::help::SUCCESSOR_GRAMMAR_VERSION;
use super::model::{CommandDescriptor, Group, OptionSpec, ValueKind};
use clap::{Arg, ArgAction, Command, value_parser};

pub(crate) const JSON_ID: &str = "successor-json";
pub(crate) const JSON_LONG: &str = "json";
pub(crate) const JSON_TOKEN: &str = "--json";
pub(crate) const HELP_ID: &str = "successor-help";
pub(crate) const VERSION_ID: &str = "successor-version";

pub fn parser_command() -> Command {
    let mut command = base_command("ultragoal")
        .about("Harness Ultragoal product-semantic successor CLI")
        .arg(
            Arg::new(JSON_ID)
                .long(JSON_LONG)
                .global(true)
                .action(ArgAction::Set)
                .num_args(0)
                .default_missing_value("true")
                .value_parser(value_parser!(bool))
                .help("Select versioned machine output"),
        )
        .arg(
            Arg::new(HELP_ID)
                .long("help")
                .short('h')
                .global(true)
                .action(ArgAction::Help)
                .help("Show catalog-derived help"),
        )
        .arg(
            Arg::new(VERSION_ID)
                .long("version")
                .global(true)
                .action(ArgAction::Version)
                .help("Show the successor grammar version"),
        );
    for group in Group::ALL {
        command = command.subcommand(group_command(group));
    }
    command
}

fn group_command(group: Group) -> Command {
    let routes: Vec<_> = catalog()
        .iter()
        .filter(|descriptor| descriptor.command.group() == group)
        .collect();
    let mut command = base_command(group.as_str()).about("Product-semantic command group");
    if let Some(default) = routes
        .iter()
        .find(|descriptor| descriptor.subcommand.is_none())
    {
        command = add_options(command.about(default.purpose), default);
    }
    for descriptor in routes
        .into_iter()
        .filter(|descriptor| descriptor.subcommand.is_some())
    {
        command = command.subcommand(route_command(descriptor));
    }
    command
}

fn route_command(descriptor: &CommandDescriptor) -> Command {
    let name = descriptor
        .subcommand
        .expect("catalog route command requires subcommand name");
    add_options(base_command(name).about(descriptor.purpose), descriptor)
}

fn add_options(mut command: Command, descriptor: &CommandDescriptor) -> Command {
    for option in descriptor.options {
        command = command.arg(option_arg(*option));
    }
    command
}

fn option_arg(option: OptionSpec) -> Arg {
    let long = option.name.as_str().trim_start_matches("--");
    let argument = Arg::new(long).long(long).required(option.required);
    match option.kind {
        ValueKind::Flag => argument
            .action(ArgAction::Set)
            .num_args(0)
            .default_missing_value("true")
            .value_parser(value_parser!(bool)),
        ValueKind::Identifier => argument.action(ArgAction::Set).num_args(1).value_name("ID"),
        ValueKind::RelativePath => argument
            .action(ArgAction::Set)
            .num_args(1)
            .value_name("RELATIVE_PATH"),
    }
}

fn base_command(name: &'static str) -> Command {
    Command::new(name)
        .version(SUCCESSOR_GRAMMAR_VERSION)
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .disable_version_flag(true)
}
