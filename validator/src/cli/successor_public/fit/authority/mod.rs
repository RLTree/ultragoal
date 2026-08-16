use crate::cli::successor::ExitClass;
use crate::cli::successor::runtime::{Diagnostic, DiagnosticId, RuntimeOutcome};
use crate::context::LiveContext;
use crate::repository_fit::{AdapterErrorId, PreparedFitApply, RepositoryFitProductionOutcome};
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryFitAuthorityMigrationAdmission {
    pub(crate) status: &'static str,
    pub(crate) authority_status: &'static str,
    pub(crate) generation: u64,
    pub(crate) event_count: usize,
    pub(crate) reservation_count: usize,
    pub(crate) nonterminal_reservation_count: usize,
    pub(crate) pending_envelope_count: usize,
    pub(crate) process_lock_count: usize,
    pub(crate) stored_root: crate::repository_fit::RepositoryFitStoredRootIdentity,
    pub(crate) current_root: crate::repository_fit::RepositoryFitStoredRootIdentity,
    pub(crate) stored_lock: crate::repository_fit::RepositoryFitStoredLockIdentity,
    pub(crate) current_lock: crate::repository_fit::RepositoryFitStoredLockIdentity,
    pub(crate) quarantine_plan: Option<RepositoryFitAuthorityQuarantinePlan>,
    pub(crate) next_action: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryFitAuthorityQuarantinePlan {
    pub(crate) plan_id: String,
    pub(crate) source_inventory_sha256: String,
    pub(crate) ledger_sha256: String,
    pub(crate) authority_id: String,
    pub(crate) ledger_head_sha256: String,
    pub(crate) source_owner: &'static str,
    pub(crate) quarantine_owner: String,
    pub(crate) target_format: &'static str,
    pub(crate) strategy: &'static str,
    pub(crate) apply_capability: &'static str,
    pub(crate) operations: Vec<&'static str>,
    pub(crate) rollback: &'static str,
}

#[cfg(target_vendor = "apple")]
#[path = "supported/mod.rs"]
mod supported;

#[path = "public_effect.rs"]
mod public_effect;
#[path = "quarantine_record.rs"]
mod quarantine_record;

pub(crate) use public_effect::*;
pub(crate) use quarantine_record::*;
#[cfg(all(test, target_vendor = "apple"))]
pub(crate) use supported::set_quarantine_failpoint_for_test;

pub(crate) fn assess_migration_admission(
    home: &Path,
) -> Result<Option<RepositoryFitAuthorityMigrationAdmission>, ()> {
    #[cfg(target_vendor = "apple")]
    {
        supported::assess_migration_admission(home).map_err(|_| ())
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = home;
        Ok(None)
    }
}
