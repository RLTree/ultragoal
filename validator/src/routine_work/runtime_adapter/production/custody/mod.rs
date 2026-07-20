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
#[cfg(all(test, target_vendor = "apple"))]
pub(crate) use store::{set_test_publication_ambiguity_after, set_test_publication_refusal_after};
pub(super) use transaction::{
    AuthorityBinding, OutputComponentJournal, OutputDirectoryIdentity, OutputProvisionJournal,
    OutputStageAmbiguity,
};
pub(super) use transaction::{mediate_reserved_effect, reconcile_reserved_effect};

#[cfg(test)]
pub(super) fn issue_test_custody(authority_root: &Path) -> RoutineCustodyCapability {
    RoutineCustodyCapability::issue_for_test(authority_root)
}
