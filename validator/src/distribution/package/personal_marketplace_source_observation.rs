use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::filesystem::ReadOnlyTreeObservation;
use std::path::Path;

pub(crate) const PERSONAL_MARKETPLACE_SOURCE_RELATIVE: &str =
    ".codex/local-marketplaces/harness-ultragoal-local/plugins/harness-ultragoal";
const MAXIMUM_SOURCE_ENTRIES: usize = 4096;
const MAXIMUM_SOURCE_BYTES: usize = 65 * 1024 * 1024;

/// Exact read-only prior-source custody for a personal Harness installation.
///
/// The inner observation owns only read descriptors and a bounded digest. It
/// cannot be converted into distribution mutation, recovery, or cleanup
/// authority.
#[derive(Debug)]
pub(crate) struct PersonalMarketplaceSourceObservation {
    tree: ReadOnlyTreeObservation,
}

impl PersonalMarketplaceSourceObservation {
    pub(crate) fn capture(home: &Path, installed_root: &Path) -> Result<Self, DistributionError> {
        let canonical_home = home
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if canonical_home != home
            || installed_root != canonical_home.join(PERSONAL_MARKETPLACE_SOURCE_RELATIVE)
        {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        let tree = ReadOnlyTreeObservation::capture(
            &canonical_home,
            PERSONAL_MARKETPLACE_SOURCE_RELATIVE,
            MAXIMUM_SOURCE_ENTRIES,
            MAXIMUM_SOURCE_BYTES,
        )?;
        Ok(Self { tree })
    }

    pub(crate) fn tree_sha256(&self) -> &str {
        self.tree.tree_sha256()
    }

    pub(crate) fn observation_sha256(&self) -> &str {
        self.tree.observation_sha256()
    }

    pub(crate) fn revalidate(&self) -> Result<(), DistributionError> {
        self.tree.revalidate()
    }
}
