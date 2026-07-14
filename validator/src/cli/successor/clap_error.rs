use super::command_contract::OutputMode;
use super::error::{ParseErrorId, ParseFailure};
use clap::error::{ContextKind, ContextValue, Error, ErrorKind};

pub(crate) fn map_clap_error(error: &Error, output_mode: OutputMode) -> ParseFailure {
    let invalid = context_text(error, ContextKind::InvalidArg)
        .or_else(|| context_text(error, ContextKind::InvalidSubcommand));
    let invalid_value = context_text(error, ContextKind::InvalidValue);
    let id = if invalid_value
        .as_deref()
        .is_some_and(|value| matches!(value, "--help" | "-h"))
    {
        ParseErrorId::HelpValueConfusion
    } else if invalid.as_deref().is_some_and(is_effect_override) {
        ParseErrorId::EffectOverrideForbidden
    } else if invalid.as_deref().is_some_and(is_implicit_write_verb) {
        ParseErrorId::ImplicitWriteVerb
    } else {
        error_id(error.kind(), invalid.as_deref())
    };
    ParseFailure::new(id, output_mode)
}

fn error_id(kind: ErrorKind, invalid: Option<&str>) -> ParseErrorId {
    match kind {
        ErrorKind::InvalidSubcommand => ParseErrorId::UnknownSubcommand,
        ErrorKind::UnknownArgument => invalid.map_or(ParseErrorId::UnknownOption, |value| {
            if value.starts_with('-') {
                ParseErrorId::UnknownOption
            } else {
                ParseErrorId::UnknownSubcommand
            }
        }),
        ErrorKind::ArgumentConflict => ParseErrorId::DuplicateOption,
        ErrorKind::MissingRequiredArgument => ParseErrorId::MissingRequiredOption,
        ErrorKind::TooFewValues | ErrorKind::WrongNumberOfValues | ErrorKind::NoEquals => {
            ParseErrorId::MissingOptionValue
        }
        ErrorKind::TooManyValues => ParseErrorId::UnexpectedOptionValue,
        ErrorKind::InvalidUtf8 => ParseErrorId::NonUtf8Argument,
        ErrorKind::InvalidValue | ErrorKind::ValueValidation => ParseErrorId::InvalidIdentifier,
        ErrorKind::MissingSubcommand => ParseErrorId::MissingSubcommand,
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        | ErrorKind::DisplayVersion
        | ErrorKind::Io
        | ErrorKind::Format => ParseErrorId::ParserFailure,
        _ => ParseErrorId::ParserFailure,
    }
}

fn context_text(error: &Error, kind: ContextKind) -> Option<String> {
    match error.get(kind)? {
        ContextValue::String(value) => Some(value.clone()),
        ContextValue::StyledStr(value) => Some(value.to_string()),
        _ => None,
    }
}

fn is_effect_override(value: &str) -> bool {
    let name = value.split_once('=').map_or(value, |(name, _)| name);
    matches!(
        name,
        "--effect"
            | "--read-only"
            | "--dry-run"
            | "--no-write"
            | "--allow-write"
            | "--write"
            | "--destructive"
            | "--force"
    )
}

fn is_implicit_write_verb(value: &str) -> bool {
    matches!(
        value,
        "write"
            | "save"
            | "execute"
            | "apply"
            | "publish"
            | "export"
            | "delete"
            | "remove"
            | "force"
            | "run"
            | "build"
            | "install"
            | "retire"
            | "promote"
            | "update"
            | "create"
    )
}
