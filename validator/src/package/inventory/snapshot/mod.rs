use super::DraftPackageManifest;
use std::collections::BTreeMap;
use std::sync::Arc;

#[cfg(unix)]
mod capture;
mod identity;
#[cfg(all(test, unix))]
mod tests;
#[cfg(unix)]
mod tree;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PackageEntryKind {
    Regular { single_link: bool },
    Directory,
    Symlink,
    Special,
}

#[derive(Debug)]
pub(crate) struct PackageSnapshot {
    context_id: Arc<str>,
    snapshot_id: Arc<str>,
    package_digest: Arc<str>,
    manifest_bytes: Arc<[u8]>,
    manifest: Arc<DraftPackageManifest>,
    listed_paths: Arc<[String]>,
    packaged_paths: Arc<[String]>,
    dependency_paths: Arc<[String]>,
    unix_modes: Arc<BTreeMap<String, u32>>,
    tree: Arc<BTreeMap<String, PackageEntryKind>>,
    bytes: Arc<BTreeMap<String, Arc<[u8]>>>,
}

impl PackageSnapshot {
    pub(crate) fn context_id(&self) -> &str {
        &self.context_id
    }

    pub(crate) fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }

    pub(crate) fn package_digest(&self) -> &str {
        &self.package_digest
    }

    pub(crate) fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }

    pub(crate) fn manifest(&self) -> &DraftPackageManifest {
        &self.manifest
    }

    pub(crate) fn listed_paths(&self) -> &[String] {
        &self.listed_paths
    }

    pub(crate) fn packaged_paths(&self) -> &[String] {
        &self.packaged_paths
    }

    pub(crate) fn dependency_paths(&self) -> &[String] {
        &self.dependency_paths
    }

    pub(crate) fn unix_mode(&self, relative: &str) -> Option<u32> {
        self.unix_modes.get(relative).copied()
    }

    pub(crate) fn tree(&self) -> &BTreeMap<String, PackageEntryKind> {
        &self.tree
    }

    pub(crate) fn bytes(&self, relative: &str) -> Option<&[u8]> {
        self.bytes.get(relative).map(AsRef::as_ref)
    }
}

#[cfg(unix)]
pub(crate) use capture::PackageCapture;

#[cfg(not(unix))]
pub(crate) struct PackageCapture;

#[cfg(not(unix))]
impl PackageCapture {
    pub(crate) fn begin(_context: &crate::context::LiveContext) -> Result<Self, String> {
        Err("package snapshot is unavailable on this platform".to_string())
    }

    pub(crate) fn finish(self) -> Result<Arc<PackageSnapshot>, String> {
        Err("package snapshot is unavailable on this platform".to_string())
    }
}
