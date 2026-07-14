//! Canonical typed grammar and fail-closed public successor runtime.
//!
//! Public routing is intentionally narrow: accepted context, inventory, and
//! typed-state reads execute; unavailable downstream adapters return stable
//! diagnostics. Retained legacy routes remain outside this module until their
//! behavioral replacements are independently accepted.

mod catalog;
mod clap_error;
mod clap_grammar;
mod command_contract;
mod command_line_input;
pub(crate) mod compatibility;
mod error;
mod help;
mod input;
mod parser;
pub(crate) mod runtime;
mod value;

pub use crate::context::EffectClass;
pub use catalog::{catalog, descriptor_for};
pub use clap_grammar::parser_command;
#[cfg(test)]
pub use command_contract::LegacyCommand;
pub use command_contract::{
    CheckProfile, CommandDescriptor, EvalAction, ExitClass, FitAction, Group, HelpTarget,
    InspectTarget, MigrateAction, ObserveAction, OptionArgument, OptionName, OptionSpec,
    OutputMode, PackageAction, ParseOutcome, ParsedCommandLine, ParsedInvocation, ParsedValue,
    RelativePath, SuccessorCommand, ValueKind, WorkspaceRoot, effect_name,
};
pub use error::ParseFailure;
#[cfg(test)]
pub use error::{ParseError, ParseErrorId};
pub use help::{render_help, version_text};
#[cfg(test)]
pub(crate) use input::MAX_ARGUMENT_BYTES;
#[cfg(test)]
pub use parser::parse_args;
pub use parser::parse_command_line;
