#[cfg(test)]
use crate::cli::successor::parse_args;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticId, RuntimeOutcome, RuntimeSession};
use crate::cli::successor::{
    CheckProfile, EffectClass, ExitClass, FitAction, InspectTarget, OutputMode, ParseOutcome,
    ParsedInvocation, SuccessorCommand, render_help, version_text,
};
use crate::context::{BuildRequest, LiveContext};
use crate::inventory::{
    ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256, AuthorityCatalog,
    InventoryBuilder,
};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[path = "output_emission.rs"]
mod output_emission;
#[path = "output_limit.rs"]
mod output_limit;

mod capabilities;
mod diagnose;
mod fit;
mod local_store;
mod observe;
mod public_context;
mod routine;
mod strict;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "diagnose_tests/mod.rs"]
mod diagnose_tests;

#[cfg(test)]
#[path = "diagnose_boundary_tests.rs"]
mod diagnose_boundary_tests;

#[cfg(test)]
mod test_support;

pub(crate) use output_emission::*;
pub(crate) use output_limit::*;
