use super::catalog::descriptor_for;
use super::clap_error::map_clap_error;
use super::clap_grammar::parser_command;
use super::command_contract::{
    CommandDescriptor, Group, OptionArgument, OutputMode, ParseOutcome, ParsedCommandLine,
    ParsedInvocation, ParsedValue, ValueKind,
};
use super::command_line_input::take_workspace_root;
use super::compatibility::classify_legacy_command;
use super::error::{ParseErrorId, ParseFailure};
use super::input::{prepare_args, requested_help_target, version_is_standalone};
use super::value::parse_value;
use clap::ArgMatches;
use clap::error::ErrorKind;
use std::ffi::OsString;

#[cfg(test)]
pub fn parse_args<I, S>(args: I) -> Result<ParseOutcome, ParseFailure>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    parse_command_line(args).map(ParsedCommandLine::into_outcome)
}

pub fn parse_command_line<I, S>(args: I) -> Result<ParsedCommandLine, ParseFailure>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let (mut args, output_mode, failure_mode) = prepare_args(args)?;
    let root = take_workspace_root(&mut args, failure_mode)?;
    parse_prepared(&args, output_mode).map(|outcome| ParsedCommandLine::new(root, outcome))
}

fn parse_prepared(
    args: &[OsString],
    output_mode: OutputMode,
) -> Result<ParseOutcome, ParseFailure> {
    if let Some(command) = classify_legacy_command(args) {
        return Ok(ParseOutcome::Compatibility {
            command,
            output_mode,
        });
    }
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push(OsString::from("ultragoal"));
    argv.extend(args.iter().cloned());
    match parser_command().try_get_matches_from(argv) {
        Ok(matches) => parse_matches(&matches, output_mode),
        Err(error) if error.kind() == ErrorKind::DisplayHelp => Ok(ParseOutcome::Help {
            target: requested_help_target(args),
            output_mode,
        }),
        Err(error) if error.kind() == ErrorKind::DisplayVersion => {
            if version_is_standalone(args) {
                Ok(ParseOutcome::Version(output_mode))
            } else {
                fail(ParseErrorId::InvalidVersionPosition, output_mode)
            }
        }
        Err(error) => Err(map_clap_error(&error, output_mode)),
    }
}

fn parse_matches(
    matches: &ArgMatches,
    output_mode: OutputMode,
) -> Result<ParseOutcome, ParseFailure> {
    let Some((group_name, group_matches)) = matches.subcommand() else {
        return fail(ParseErrorId::EmptyInvocation, output_mode);
    };
    let group =
        Group::parse(group_name).ok_or_else(|| failure(ParseErrorId::UnknownGroup, output_mode))?;
    let (descriptor, active_matches) = match group_matches.subcommand() {
        Some((subcommand, child_matches)) => (
            descriptor_for(group, Some(subcommand))
                .ok_or_else(|| failure(ParseErrorId::UnknownSubcommand, output_mode))?,
            child_matches,
        ),
        None => (
            descriptor_for(group, None)
                .ok_or_else(|| failure(ParseErrorId::MissingSubcommand, output_mode))?,
            group_matches,
        ),
    };
    let arguments = descriptor_arguments(descriptor, active_matches, output_mode)?;
    Ok(ParseOutcome::Invocation(ParsedInvocation {
        command: descriptor.command,
        effect: descriptor.effect,
        output_mode,
        arguments,
    }))
}

fn descriptor_arguments(
    descriptor: &CommandDescriptor,
    matches: &ArgMatches,
    output_mode: OutputMode,
) -> Result<Vec<OptionArgument>, ParseFailure> {
    let mut arguments = Vec::new();
    for spec in descriptor.options {
        let key = spec.name.as_str().trim_start_matches("--");
        let parsed = match spec.kind {
            ValueKind::Flag => matches
                .get_one::<bool>(key)
                .copied()
                .filter(|value| *value)
                .map(|_| ParsedValue::Flag),
            kind => matches
                .get_one::<String>(key)
                .map(|value| parse_value(kind, value))
                .transpose()
                .map_err(|id| failure(id, output_mode))?,
        };
        if let Some(value) = parsed {
            arguments.push(OptionArgument {
                name: spec.name,
                value,
            });
        }
    }
    Ok(arguments)
}

const fn failure(id: ParseErrorId, output_mode: OutputMode) -> ParseFailure {
    ParseFailure::new(id, output_mode)
}

fn fail<T>(id: ParseErrorId, output_mode: OutputMode) -> Result<T, ParseFailure> {
    Err(failure(id, output_mode))
}
