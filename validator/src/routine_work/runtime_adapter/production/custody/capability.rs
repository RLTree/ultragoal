use super::*;
use crate::cli::successor_public::routine::HostCustodyIssuance;

/// Opaque authority to enter the private routine custody transaction.
///
/// The host adapter issues this non-clone capability after opening and
/// validating its anchored state. Only the private custody owner can recover
/// the authority path.
pub(crate) struct RoutineCustodyCapability {
    authority_root: PathBuf,
    _seal: CapabilitySeal,
}

struct CapabilitySeal;

impl RoutineCustodyCapability {
    pub(crate) fn issue_from_host(issuance: HostCustodyIssuance) -> Self {
        Self {
            authority_root: issuance.into_authority_root(),
            _seal: CapabilitySeal,
        }
    }

    #[cfg(test)]
    pub(super) fn issue_for_test(authority_root: &Path) -> Self {
        Self::issue_from_host(HostCustodyIssuance::for_test(authority_root))
    }

    pub(in crate::routine_work::runtime_adapter::production) fn authority_root(&self) -> &Path {
        &self.authority_root
    }
}
