use crate::cli::successor::ExitClass;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticId, RuntimeOutcome};
use crate::context::LiveContext;
use crate::repository_fit::{AdapterErrorId, PreparedFitApply, RepositoryFitProductionOutcome};
use std::path::Path;

#[cfg(target_vendor = "apple")]
#[path = "supported/mod.rs"]
mod supported;

#[path = "public_effect.rs"]
mod public_effect;

pub(crate) use public_effect::*;
