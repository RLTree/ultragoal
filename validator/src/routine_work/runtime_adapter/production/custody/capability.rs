use super::*;

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
    pub(crate) fn issue_from_host(authority_root: PathBuf) -> Self {
        Self {
            authority_root,
            _seal: CapabilitySeal,
        }
    }

    #[cfg(test)]
    pub(super) fn issue_for_test(authority_root: &Path) -> Self {
        Self::issue_from_host(authority_root.to_path_buf())
    }

    pub(in crate::routine_work::runtime_adapter::production) fn authority_root(&self) -> &Path {
        &self.authority_root
    }
}
