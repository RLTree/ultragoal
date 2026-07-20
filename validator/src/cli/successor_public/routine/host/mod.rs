#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

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

#[cfg(target_vendor = "apple")]
#[path = "supported/mod.rs"]
mod supported;

pub(crate) use supported::ContinuationCheckpoint;

#[path = "host_failure.rs"]
mod host_failure;

#[path = "checkpoint_request.rs"]
mod checkpoint_request;

pub(crate) use checkpoint_request::*;
pub(crate) use host_failure::*;
