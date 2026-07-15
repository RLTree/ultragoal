use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HostFailure {
    #[cfg(not(target_vendor = "apple"))]
    Unsupported,
    Unavailable,
    Invalid,
    RandomUnavailable,
    ClockUnavailable,
    Persistence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CacheBinding {
    pub(crate) command: String,
    pub(crate) target_id: String,
    pub(crate) source_id: String,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) graph_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) plan_id: String,
    pub(crate) protocol_id: String,
    pub(crate) request_id: String,
}

pub(crate) struct HostState {
    #[cfg(target_vendor = "apple")]
    pub(crate) inner: supported::HostState,
}

impl HostState {
    pub(crate) fn open(home: &Path, target: &Path) -> Result<Self, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            return supported::HostState::open(home, target).map(|inner| Self { inner });
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (home, target);
            Err(HostFailure::Unsupported)
        }
    }

    pub(crate) fn target_id(&self) -> &str {
        #[cfg(target_vendor = "apple")]
        {
            &self.inner.target_id
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            unreachable!("unsupported host state cannot be constructed")
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

    pub(crate) fn read_reuse(
        &self,
        expected: &CacheBinding,
    ) -> Result<Option<Vec<Vec<u8>>>, HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.read_reuse(expected)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = expected;
            unreachable!("unsupported host state cannot be constructed")
        }
    }

    pub(crate) fn persist_reuse(
        &self,
        binding: CacheBinding,
        artifacts: &[Vec<u8>],
    ) -> Result<(), HostFailure> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.persist_reuse(binding, artifacts)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (binding, artifacts);
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
