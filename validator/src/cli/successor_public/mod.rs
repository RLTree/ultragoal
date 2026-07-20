#[cfg(test)]
use crate::cli::successor::parse_args;
use crate::cli::successor::runtime::{
    Diagnostic, DiagnosticDetails, DiagnosticId, RuntimeOutcome, RuntimeSession,
};
use crate::cli::successor::{
    EffectClass, ExitClass, FitAction, InspectTarget, OutputMode, ParseOutcome, ParsedInvocation,
    SuccessorCommand, render_help, version_text,
};
use crate::context::{BuildRequest, LiveContext};
use crate::inventory::{
    ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256, AuthorityCatalog,
    InventoryBuilder,
};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub(crate) struct HostCustodyIssuance {
    authority_root: PathBuf,
    _seal: HostCustodySeal,
}

struct HostCustodySeal;

impl HostCustodyIssuance {
    fn new(authority_root: PathBuf) -> Self {
        Self {
            authority_root,
            _seal: HostCustodySeal,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(authority_root: &Path) -> Self {
        Self::new(authority_root.to_path_buf())
    }

    pub(crate) fn into_authority_root(self) -> PathBuf {
        self.authority_root
    }
}

#[path = "output_emission.rs"]
mod output_emission;
#[path = "output_limit.rs"]
mod output_limit;

mod capabilities;
mod diagnose;
mod evaluation;
mod fit;
mod local_store;
mod migration;
mod observe;
mod operation_binding;
mod orchestration;
mod package_build;
mod package_dispatch;
mod package_install_test;
mod package_inventory;
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
mod repository_fixture;

pub(crate) use operation_binding::{active_api_identifiers, active_command_groups};
pub(crate) use output_emission::*;
pub(crate) use output_limit::*;
