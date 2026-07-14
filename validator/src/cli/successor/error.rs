use super::command_contract::{ExitClass, OutputMode};
use std::fmt;

pub const MACHINE_ERROR_SCHEMA: &str = "harness-ultragoal.cli-error.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseErrorId {
    EmptyInvocation,
    UnknownGroup,
    MissingSubcommand,
    UnknownSubcommand,
    ImplicitWriteVerb,
    UnknownOption,
    MissingOptionValue,
    DuplicateOption,
    UnexpectedOptionValue,
    InvalidPath,
    InvalidIdentifier,
    HelpValueConfusion,
    EffectOverrideForbidden,
    MissingRequiredOption,
    NonUtf8Argument,
    ArgumentTooLarge,
    TooManyArguments,
    ParserFailure,
    InvalidHelpPosition,
    InvalidVersionPosition,
}

impl ParseErrorId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyInvocation => "CLI_EMPTY_INVOCATION",
            Self::UnknownGroup => "CLI_UNKNOWN_GROUP",
            Self::MissingSubcommand => "CLI_MISSING_SUBCOMMAND",
            Self::UnknownSubcommand => "CLI_UNKNOWN_SUBCOMMAND",
            Self::ImplicitWriteVerb => "CLI_IMPLICIT_WRITE_VERB",
            Self::UnknownOption => "CLI_UNKNOWN_OPTION",
            Self::MissingOptionValue => "CLI_MISSING_OPTION_VALUE",
            Self::DuplicateOption => "CLI_DUPLICATE_SINGLETON_OPTION",
            Self::UnexpectedOptionValue => "CLI_UNEXPECTED_OPTION_VALUE",
            Self::InvalidPath => "CLI_INVALID_CONFINED_PATH",
            Self::InvalidIdentifier => "CLI_INVALID_IDENTIFIER",
            Self::HelpValueConfusion => "CLI_HELP_OPTION_VALUE_CONFUSION",
            Self::EffectOverrideForbidden => "CLI_EFFECT_OVERRIDE_FORBIDDEN",
            Self::MissingRequiredOption => "CLI_MISSING_REQUIRED_OPTION",
            Self::NonUtf8Argument => "CLI_NON_UTF8_ARGUMENT",
            Self::ArgumentTooLarge => "CLI_ARGUMENT_TOO_LARGE",
            Self::TooManyArguments => "CLI_TOO_MANY_ARGUMENTS",
            Self::ParserFailure => "CLI_PARSER_INTERNAL_FAILURE",
            Self::InvalidHelpPosition => "CLI_INVALID_HELP_POSITION",
            Self::InvalidVersionPosition => "CLI_INVALID_VERSION_POSITION",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::EmptyInvocation => "a product-semantic command group is required",
            Self::UnknownGroup => "the command group is not in the successor catalog",
            Self::MissingSubcommand => "this command group requires an explicit subcommand",
            Self::UnknownSubcommand => "the subcommand is not in the successor catalog",
            Self::ImplicitWriteVerb => "an undeclared write verb cannot select an effect",
            Self::UnknownOption => "the option is not valid for the selected command",
            Self::MissingOptionValue => "a singleton option value is missing",
            Self::DuplicateOption => "a singleton option was provided more than once",
            Self::UnexpectedOptionValue => "a flag does not accept a value",
            Self::InvalidPath => "a path must be normalized and confined relative to the root",
            Self::InvalidIdentifier => {
                "an identifier is empty, oversized, or contains unsafe bytes"
            }
            Self::HelpValueConfusion => "help cannot be consumed as another option's value",
            Self::EffectOverrideForbidden => "effect class is structural and cannot be overridden",
            Self::MissingRequiredOption => "the selected command is missing a required option",
            Self::NonUtf8Argument => "command grammar accepts only bounded UTF-8 arguments",
            Self::ArgumentTooLarge => "an argument exceeds the parser's fixed byte bound",
            Self::TooManyArguments => "the invocation exceeds the parser's fixed argument bound",
            Self::ParserFailure => "the typed parser failed outside a valid invocation class",
            Self::InvalidHelpPosition => "help must be a standalone root or command request",
            Self::InvalidVersionPosition => "version must be a standalone root request",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseError {
    id: ParseErrorId,
}

impl ParseError {
    pub const fn new(id: ParseErrorId) -> Self {
        Self { id }
    }

    pub const fn id(self) -> ParseErrorId {
        self.id
    }

    pub const fn exit_class(self) -> ExitClass {
        match self.id {
            ParseErrorId::ParserFailure => ExitClass::InternalFailure,
            _ => ExitClass::InvalidInvocation,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.id.as_str(), self.id.message())
    }
}

impl std::error::Error for ParseError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseFailure {
    pub error: ParseError,
    pub output_mode: OutputMode,
}

impl ParseFailure {
    pub(crate) const fn new(id: ParseErrorId, output_mode: OutputMode) -> Self {
        Self {
            error: ParseError::new(id),
            output_mode,
        }
    }

    pub fn render(self) -> String {
        match self.output_mode {
            OutputMode::Human => self.error.to_string(),
            OutputMode::Json => format!(
                "{{\"schema_version\":\"{MACHINE_ERROR_SCHEMA}\",\"error_id\":\"{}\",\"exit_code\":{}}}",
                self.error.id().as_str(),
                self.error.exit_class().code()
            ),
        }
    }
}
