use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HostFailure {
    #[cfg(not(target_vendor = "apple"))]
    Unsupported,
    Unavailable,
    Busy,
    Invalid,
}

pub(crate) struct HostState {
    #[cfg(target_vendor = "apple")]
    pub(crate) inner: supported::HostState,
}

impl HostState {
    pub(crate) fn open_or_bootstrap(home: &Path, target: &Path) -> Result<Self, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            supported::HostState::open_or_bootstrap(home, target).map(|inner| Self { inner })
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (home, target);
            Err(HostFailure::Unsupported)
        }
    }

    pub(crate) fn authority_root(&self) -> &Path {
        #[cfg(target_vendor = "apple")]
        {
            &self.inner.authority.path
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn verify(&self) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.verify()
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            unreachable!("unsupported host state cannot be constructed")
        }
    }
}
