//! Root-owned repository-fit public adapter surface.
//!
//! Read routes remain projection-only. Apply consumes one exact accepted plan,
//! persists restart authority in preprovisioned owner-only host state, and then
//! enters the sealed repository-fit production kernel.

use crate::cli::successor::runtime::{Diagnostic, DiagnosticDetails, DiagnosticId, RuntimeOutcome};
use crate::cli::successor::{
    EffectClass, ExitClass, FitAction, OptionName, ParsedInvocation, ParsedValue, SuccessorCommand,
};
use crate::context::LiveContext;
use crate::repository_fit::{
    AdapterErrorId, FitAdapterError, FitPlanScope, PreparedFitApply, inspect_target, plan_target,
    plan_target_for_scope, prepare_apply_request, verify_target,
};
use std::path::{Path, PathBuf};

mod authority;
mod external_plan_file;
#[path = "invocation_errors.rs"]
mod invocation_errors;
#[path = "plan_input_limit.rs"]
mod plan_input_limit;

pub(crate) use invocation_errors::*;
pub(crate) use plan_input_limit::*;
