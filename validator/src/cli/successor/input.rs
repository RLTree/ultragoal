use super::catalog::{catalog, descriptor_for};
use super::clap_grammar::{JSON_TOKEN, ROOT_TOKEN};
use super::command_contract::{Group, HelpTarget, OutputMode, ValueKind};
use super::error::{ParseErrorId, ParseFailure};
use std::ffi::{OsStr, OsString};

pub(crate) const MAX_ARGUMENTS: usize = 256;
pub(crate) const MAX_ARGUMENT_BYTES: usize = 8192;

pub(crate) fn prepare_args<I, S>(
    args: I,
) -> Result<(Vec<OsString>, OutputMode, OutputMode), ParseFailure>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut collected = Vec::new();
    for argument in args {
        let argument = argument.into();
        if collected.len() == MAX_ARGUMENTS {
            let mode = machine_failure_mode(
                collected
                    .iter()
                    .map(OsString::as_os_str)
                    .chain(std::iter::once(argument.as_os_str())),
            );
            return fail(ParseErrorId::TooManyArguments, mode);
        }
        collected.push(argument);
    }
    let failure_mode = machine_failure_mode(collected.iter().map(OsString::as_os_str));
    let mut output_mode = OutputMode::Human;
    let mut help_seen = false;
    for argument in &collected {
        let value = argument
            .to_str()
            .ok_or_else(|| failure(ParseErrorId::NonUtf8Argument, failure_mode))?;
        if value.len() > MAX_ARGUMENT_BYTES {
            return fail(ParseErrorId::ArgumentTooLarge, failure_mode);
        }
        if value == JSON_TOKEN && !help_seen {
            output_mode = OutputMode::Json;
        }
        help_seen |= is_help_token(value);
    }
    validate_preparse_guards(semantic_prefix(&collected), output_mode)?;
    Ok((collected, output_mode, failure_mode))
}

fn machine_failure_mode<'a>(args: impl IntoIterator<Item = &'a OsStr>) -> OutputMode {
    if args.into_iter().any(|argument| argument == JSON_TOKEN) {
        OutputMode::Json
    } else {
        OutputMode::Human
    }
}

pub(crate) fn requested_help_target(args: &[OsString]) -> HelpTarget {
    let mut prefix = args
        .iter()
        .filter_map(|argument| argument.to_str())
        .take_while(|token| !is_help_token(token))
        .filter(|token| *token != JSON_TOKEN);
    let Some(group) = prefix.next().and_then(Group::parse) else {
        return HelpTarget::Root;
    };
    prefix
        .next()
        .and_then(|subcommand| descriptor_for(group, Some(subcommand)))
        .map_or(HelpTarget::Group(group), |descriptor| {
            HelpTarget::Command(descriptor.command)
        })
}

pub(crate) fn version_is_standalone(args: &[OsString]) -> bool {
    args.iter()
        .all(|argument| matches!(argument.to_str(), Some("--version" | "--json")))
}

fn validate_preparse_guards(
    args: &[OsString],
    output_mode: OutputMode,
) -> Result<(), ParseFailure> {
    let text: Vec<_> = args
        .iter()
        .filter_map(|argument| argument.to_str())
        .collect();
    if has_help_value_confusion(&text) {
        return fail(ParseErrorId::HelpValueConfusion, output_mode);
    }
    if has_missing_option_value(&text) {
        return fail(ParseErrorId::MissingOptionValue, output_mode);
    }
    if has_flag_value(&text) {
        return fail(ParseErrorId::UnexpectedOptionValue, output_mode);
    }
    for singleton in ["--json", "--root", "--help", "-h", "--version"] {
        if text.iter().filter(|token| **token == singleton).count() > 1 {
            return fail(ParseErrorId::DuplicateOption, output_mode);
        }
    }
    let has_help = text.iter().any(|token| matches!(*token, "--help" | "-h"));
    let has_version = text.contains(&"--version");
    if has_help && has_version {
        return fail(ParseErrorId::InvalidHelpPosition, output_mode);
    }
    Ok(())
}

fn semantic_prefix(args: &[OsString]) -> &[OsString] {
    let end = args
        .iter()
        .position(|argument| argument.to_str().is_some_and(is_help_token))
        .map_or(args.len(), |index| index + 1);
    &args[..end]
}

fn is_help_token(token: &str) -> bool {
    matches!(token, "--help" | "-h")
}

fn has_help_value_confusion(text: &[&str]) -> bool {
    text.windows(2)
        .any(|pair| is_value_option(pair[0]) && matches!(pair[1], "--help" | "-h"))
        || text.iter().any(|token| {
            token.split_once('=').is_some_and(|(name, value)| {
                is_value_option(name) && matches!(value, "--help" | "-h")
            })
        })
}

fn has_missing_option_value(text: &[&str]) -> bool {
    text.iter().enumerate().any(|(index, token)| {
        is_value_option(token)
            && text
                .get(index + 1)
                .is_none_or(|value| value.starts_with('-'))
    })
}

fn has_flag_value(text: &[&str]) -> bool {
    text.iter().any(|token| {
        token
            .split_once('=')
            .is_some_and(|(name, _)| is_flag_option(name))
    })
}

fn is_value_option(token: &str) -> bool {
    token == ROOT_TOKEN
        || catalog().iter().any(|descriptor| {
            descriptor
                .options
                .iter()
                .any(|option| option.kind != ValueKind::Flag && option.name.as_str() == token)
        })
}

fn is_flag_option(token: &str) -> bool {
    catalog().iter().any(|descriptor| {
        descriptor
            .options
            .iter()
            .any(|option| option.kind == ValueKind::Flag && option.name.as_str() == token)
    })
}

const fn failure(id: ParseErrorId, output_mode: OutputMode) -> ParseFailure {
    ParseFailure::new(id, output_mode)
}

fn fail<T>(id: ParseErrorId, output_mode: OutputMode) -> Result<T, ParseFailure> {
    Err(failure(id, output_mode))
}
