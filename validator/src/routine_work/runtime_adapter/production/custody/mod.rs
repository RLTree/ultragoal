//! Private durable custody for one routine production transaction.

use super::*;

#[path = "capability.rs"]
mod capability;
#[path = "observations.rs"]
mod observations;
#[path = "store/mod.rs"]
mod store;
#[path = "transaction/mod.rs"]
mod transaction;

pub(crate) use capability::RoutineCustodyCapability;
use store::DurableCustody;
#[cfg(all(test, target_vendor = "apple"))]
pub(crate) use store::{set_test_publication_ambiguity_after, set_test_publication_refusal_after};
pub(super) use transaction::{
    AuthorityBinding, OutputComponentJournal, OutputDirectoryIdentity, OutputProvisionJournal,
    OutputStageAmbiguity,
};
pub(super) use transaction::{mediate_reserved_effect, reconcile_reserved_effect};

pub(super) fn authenticate_public_checkpoint(
    custody: RoutineCustodyCapability,
    target: &Path,
    context_id: &str,
    candidate_id: &str,
    plan_id: &str,
    snapshot_id: &str,
    continuation: &str,
    recovery_marker: &str,
    attempt_grant: &str,
    authenticated_ledger_head: &str,
    state: &str,
    terminal_outcome: Option<&str>,
    allow_stale_head: bool,
) -> Result<(), RoutineError> {
    DurableCustody::authenticate_public_checkpoint(
        &custody,
        target,
        context_id,
        candidate_id,
        plan_id,
        snapshot_id,
        continuation,
        recovery_marker,
        attempt_grant,
        authenticated_ledger_head,
        state,
        terminal_outcome,
        allow_stale_head,
    )
}

#[cfg(test)]
pub(super) fn issue_test_custody(authority_root: &Path) -> RoutineCustodyCapability {
    RoutineCustodyCapability::issue_for_test(authority_root)
}
