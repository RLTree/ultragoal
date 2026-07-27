use super::{PackageEntryKind, PackageSnapshot, identity, tree};
use crate::context::{EffectClass, LiveContext};
use crate::package::inventory::anchored::{self, Session};
use crate::package::inventory::generated_disposition::{self, Catalog, Classification, Source};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const MANIFEST_PATH: &str = "plugin-manifest-draft.json";
const MARKETPLACE_CATALOG_PATH: &str = ".agents/plugins/marketplace.json";
const MAX_MANIFEST_PATHS: usize = 100_000;

pub(crate) struct PackageCapture {
    context: LiveContext,
    session: Session,
    snapshot: Arc<PackageSnapshot>,
}

struct CachedSource<'session> {
    session: &'session mut Session,
    bytes: BTreeMap<String, Arc<[u8]>>,
    unix_modes: BTreeMap<String, u32>,
}

struct PackageRead {
    manifest_bytes: Arc<[u8]>,
    manifest: super::super::DraftPackageManifest,
    listed_paths: Vec<String>,
    packaged_paths: Vec<String>,
    package_digest: String,
}

impl PackageCapture {
    pub(crate) fn begin(context: &LiveContext) -> Result<Self, String> {
        context
            .effect()
            .authorize(EffectClass::Read)
            .map_err(|_| "package snapshot read effect is unavailable".to_string())?;
        context
            .revalidate()
            .map_err(|_| "package snapshot live context is stale".to_string())?;
        let mut session = Session::open(context.worktree_root())
            .map_err(|_| "package snapshot root is unavailable".to_string())?;
        let tree = tree::capture(context.worktree_root(), &mut session)?;
        session.seal_root_snapshot()?;
        let mut source = CachedSource {
            session: &mut session,
            bytes: BTreeMap::new(),
            unix_modes: BTreeMap::new(),
        };
        let package = read_package(&tree, &mut source)?;
        cache_rust_sources(&tree, &mut source)?;
        if package
            .listed_paths
            .iter()
            .any(|path| !source.bytes.contains_key(path))
        {
            return Err("package snapshot listed member is unavailable".to_string());
        }
        let unix_modes = package
            .packaged_paths
            .iter()
            .map(|path| {
                source
                    .unix_modes
                    .get(path)
                    .copied()
                    .map(|mode| (path.clone(), mode))
                    .ok_or_else(|| "package snapshot packaged mode is unavailable".to_string())
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let bytes = source.bytes;
        let tree_sha256 = identity::tree(&tree);
        let dependency_sha256 = identity::dependencies(&bytes);
        let snapshot_id = identity::snapshot(
            context.context_id(),
            package.manifest_bytes.as_ref(),
            &tree_sha256,
            &dependency_sha256,
            &package.package_digest,
        );
        let snapshot = Arc::new(PackageSnapshot {
            context_id: Arc::from(context.context_id()),
            snapshot_id: Arc::from(snapshot_id),
            manifest: Arc::new(package.manifest),
            packaged_paths: Arc::from(package.packaged_paths),
            unix_modes: Arc::new(unix_modes),
            #[cfg(test)]
            tree: Arc::new(tree),
            bytes: Arc::new(bytes),
        });
        Ok(Self {
            context: context.clone(),
            session,
            snapshot,
        })
    }

    pub(crate) fn finish(self) -> Result<Arc<PackageSnapshot>, String> {
        self.session
            .finish()
            .map_err(|_| "package snapshot changed before finalization".to_string())?;
        self.context
            .revalidate()
            .map_err(|_| "package snapshot live context changed before finalization".to_string())?;
        Ok(self.snapshot)
    }
}

impl Source for CachedSource<'_> {
    fn read(&mut self, relative: &str, maximum: u64) -> Result<Arc<[u8]>, String> {
        if let Some(bytes) = self.bytes.get(relative) {
            if bytes.len() as u64 > maximum {
                return Err("package snapshot cached file exceeds its byte limit".to_string());
            }
            return Ok(Arc::clone(bytes));
        }
        if super::super::package_path_syntax_error(relative).is_some() {
            return Err("package snapshot path is invalid".to_string());
        }
        let (bytes, unix_mode) = self
            .session
            .read_with_mode(relative, maximum)
            .map_err(|_| "package snapshot file is unavailable".to_string())?;
        let bytes: Arc<[u8]> = Arc::from(bytes);
        if self
            .unix_modes
            .insert(relative.to_string(), unix_mode)
            .is_some()
        {
            return Err("package snapshot mode was captured more than once".to_string());
        }
        self.bytes.insert(relative.to_string(), Arc::clone(&bytes));
        Ok(bytes)
    }
}

mod manifest;
#[cfg(test)]
pub(super) use manifest::supported_skill_root;
use manifest::{cache_rust_sources, read_package};
