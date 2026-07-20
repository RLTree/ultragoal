//! Root-owned repository-fit public adapter surface.
//!
//! Read routes remain projection-only. Apply consumes one exact accepted plan,
//! persists restart authority in preprovisioned owner-only host state, and then
//! enters the sealed repository-fit production kernel.

use crate::cli::successor::runtime::{Diagnostic, DiagnosticId, RuntimeOutcome};
use crate::cli::successor::{
    EffectClass, ExitClass, FitAction, OptionName, ParsedInvocation, ParsedValue, SuccessorCommand,
};
use crate::context::LiveContext;
use crate::repository_fit::{
    AdapterErrorId, FitAdapterError, PreparedFitApply, inspect_target, plan_target,
    prepare_apply_request, verify_target,
};
use std::path::{Path, PathBuf};

mod authority;
#[path = "external_plan_file.rs"]
mod external_plan_file;
#[cfg(test)]
#[path = "external_plan_file_tests.rs"]
mod external_plan_file_tests;
#[path = "invocation_errors.rs"]
mod invocation_errors;
#[path = "plan_input_limit.rs"]
mod plan_input_limit;

pub(crate) use invocation_errors::*;
pub(crate) use plan_input_limit::*;
