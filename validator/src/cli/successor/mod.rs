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
mod error;
mod help;
mod input;
mod parser;
pub(crate) mod runtime;
mod value;

pub use crate::context::EffectClass;
pub use catalog::{catalog, descriptor_for};
pub use clap_grammar::parser_command;
pub use command_contract::{
    CheckProfile, CommandDescriptor, EvalAction, ExitClass, FitAction, Group, HelpTarget,
    InspectTarget, MigrateAction, ObserveAction, OptionArgument, OptionName, OptionSpec,
    OutputMode, PackageAction, ParseOutcome, ParsedInvocation, ParsedValue, RelativePath,
    SuccessorCommand, ValueKind, effect_name,
};
pub use error::{ParseError, ParseErrorId, ParseFailure};
pub use help::{render_help, version_text};
pub(crate) use input::MAX_ARGUMENT_BYTES;
pub use parser::parse_args;
