//! Canonical typed grammar and fail-closed public successor runtime.
//!
//! Public routing is intentionally narrow: accepted context, inventory, and
//! typed-state reads execute; unavailable downstream adapters return stable
//! diagnostics. Retained legacy routes remain outside this module until their
//! behavioral replacements are independently accepted.

mod catalog;
mod clap_error;
pub(crate) mod clap_grammar;
pub(crate) mod command_contract;
mod command_line_input;
pub(crate) mod compatibility;
mod error;
mod help;
mod input;
mod parser;
pub(crate) mod runtime;
mod value;

pub use crate::context::EffectClass;
pub use catalog::catalog;
#[cfg(test)]
pub use command_contract::OptionArgument;
pub use command_contract::{
    CheckProfile, ExitClass, FitAction, Group, InspectTarget, ObserveAction, OptionName,
    OptionSpec, OutputMode, ParseOutcome, ParsedCommandLine, ParsedInvocation, ParsedValue,
    SuccessorCommand, ValueKind, WorkspaceRoot, effect_name,
};
#[cfg(test)]
pub use error::ParseErrorId;
pub use error::ParseFailure;
pub(crate) use help::{HELP_SCHEMA, SUCCESSOR_GRAMMAR_VERSION};
pub use help::{render_help, version_text};
#[cfg(test)]
pub use parser::parse_args;
pub use parser::parse_command_line;
