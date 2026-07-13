use super::{PackageEntryKind, PackageSnapshot, identity, tree};
use crate::context::{EffectClass, LiveContext};
use crate::package::inventory::anchored::{self, Session};
use crate::package::inventory::generated_disposition::{self, Catalog, Classification, Source};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const MANIFEST_PATH: &str = "plugin-manifest-draft.json";
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
    manifest: serde_json::Value,
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
        let dependency_paths = bytes.keys().cloned().collect::<Vec<_>>();
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
            package_digest: Arc::from(package.package_digest),
            manifest_bytes: package.manifest_bytes,
            manifest: Arc::new(package.manifest),
            listed_paths: Arc::from(package.listed_paths),
            packaged_paths: Arc::from(package.packaged_paths),
            dependency_paths: Arc::from(dependency_paths),
            unix_modes: Arc::new(unix_modes),
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

fn read_package(
    tree: &BTreeMap<String, PackageEntryKind>,
    source: &mut CachedSource<'_>,
) -> Result<PackageRead, String> {
    let manifest_bytes = source.read(MANIFEST_PATH, anchored::MAX_MANIFEST_BYTES)?;
    let manifest = anchored::parse_unique_json(manifest_bytes.as_ref())?;
    validate_manifest_collections(&manifest)?;
    let mut listed_paths = super::super::inventory_paths(&manifest)
        .into_iter()
        .filter(|path| !super::super::package_digest_excluded(path))
        .collect::<Vec<_>>();
    if listed_paths.len() > MAX_MANIFEST_PATHS {
        return Err("package snapshot manifest exceeds its path limit".to_string());
    }
    listed_paths.sort();
    if listed_paths.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("package snapshot manifest contains a duplicate path".to_string());
    }
    if listed_paths
        .iter()
        .any(|path| super::super::package_path_syntax_error(path).is_some())
    {
        return Err("package snapshot manifest contains an invalid path".to_string());
    }
    let dispositions = Catalog::for_manifest_from(source, &listed_paths)?;
    let mut rows = Vec::new();
    if dispositions.is_some() {
        rows.push((
            generated_disposition::REGISTRY_PATH.to_string(),
            source.read(
                generated_disposition::REGISTRY_PATH,
                anchored::MAX_RESOURCE_BYTES,
            )?,
        ));
    }
    for relative in &listed_paths {
        if generated_disposition::generated_path(relative) {
            let catalog = dispositions
                .as_ref()
                .expect("generated paths require a disposition catalog");
            match catalog.classify_from(source, relative)? {
                Classification::CanonicalProjection { bytes } => {
                    rows.push((relative.clone(), bytes));
                }
                Classification::RetainedContext { .. } => {}
            }
        } else if relative != generated_disposition::REGISTRY_PATH || dispositions.is_none() {
            rows.push((
                relative.clone(),
                source.read(relative, anchored::MAX_RESOURCE_BYTES)?,
            ));
        }
    }
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    let package_digest = digest_rows(&rows)?;
    let packaged_paths = capture_supported_package_paths(tree, &manifest, source)?;
    Ok(PackageRead {
        manifest_bytes,
        manifest,
        listed_paths,
        packaged_paths,
        package_digest,
    })
}

fn capture_supported_package_paths(
    tree: &BTreeMap<String, PackageEntryKind>,
    manifest: &serde_json::Value,
    source: &mut CachedSource<'_>,
) -> Result<Vec<String>, String> {
    let supported_manifest = ".codex-plugin/plugin.json";
    if !tree
        .get(supported_manifest)
        .is_some_and(|kind| matches!(kind, PackageEntryKind::Regular { single_link: true }))
    {
        return Err("package snapshot supported manifest is unavailable".to_string());
    }
    source.read(supported_manifest, anchored::MAX_MANIFEST_BYTES)?;

    let skills = manifest
        .get("skills")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "package snapshot manifest collection is missing".to_string())?;
    let mut roots = Vec::with_capacity(skills.len());
    for row in skills {
        let path = row
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "package snapshot manifest component path is invalid".to_string())?;
        let root = path
            .strip_prefix("skills/")
            .and_then(|rest| rest.strip_suffix("/SKILL.md"))
            .filter(|name| !name.is_empty() && !name.contains('/'))
            .map(|name| format!("skills/{name}"))
            .ok_or_else(|| "package snapshot skill root is invalid".to_string())?;
        roots.push(root);
    }
    roots.sort();
    let mut folded_roots = BTreeSet::new();
    if roots.windows(2).any(|pair| pair[0] == pair[1])
        || roots
            .iter()
            .any(|root| !folded_roots.insert(root.to_ascii_lowercase()))
    {
        return Err("package snapshot skill roots are not unique".to_string());
    }

    let mut required = BTreeSet::from([supported_manifest.to_string()]);
    for root in &roots {
        required.insert(format!("{root}/SKILL.md"));
        required.insert(format!("{root}/agents/openai.yaml"));
    }

    let mut packaged = vec![supported_manifest.to_string()];
    for (path, kind) in tree {
        let exact_root = supported_skill_root(path, &roots)?;
        if exact_root.is_none() {
            continue;
        }
        if matches!(kind, PackageEntryKind::Directory) {
            continue;
        }
        if !required.contains(path) {
            return Err("package snapshot skill subtree contains an unknown member".to_string());
        }
        match kind {
            PackageEntryKind::Regular { single_link: true } => {
                source.read(path, anchored::MAX_RESOURCE_BYTES)?;
                packaged.push(path.clone());
            }
            PackageEntryKind::Directory => unreachable!("directories are handled above"),
            PackageEntryKind::Regular { single_link: false }
            | PackageEntryKind::Symlink
            | PackageEntryKind::Special => {
                return Err("package snapshot skill subtree is unsafe".to_string());
            }
        }
    }
    packaged.sort();
    if packaged.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("package snapshot packaged paths are not unique".to_string());
    }
    if packaged.len() != required.len()
        || required
            .iter()
            .any(|expected| packaged.binary_search(expected).is_err())
    {
        return Err("package snapshot canonical package member is missing".to_string());
    }
    Ok(packaged)
}

pub(super) fn supported_skill_root<'a>(
    path: &str,
    roots: &'a [String],
) -> Result<Option<&'a str>, String> {
    let exact_root = roots
        .iter()
        .find(|root| path.starts_with(&(root.to_string() + "/")));
    let folded = path.to_ascii_lowercase();
    let folded_root = roots
        .iter()
        .find(|root| folded.starts_with(&(root.to_ascii_lowercase() + "/")));
    if exact_root.is_none() && folded_root.is_some() {
        return Err("package snapshot skill subtree has a case collision".to_string());
    }
    Ok(exact_root.map(String::as_str))
}

fn digest_rows(rows: &[(String, Arc<[u8]>)]) -> Result<String, String> {
    let mut hasher = Sha256::new();
    for (relative, bytes) in rows {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(super::super::payload::stable_package_payload(
            relative,
            bytes.as_ref(),
        )?);
        hasher.update([0]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn cache_rust_sources(
    tree: &BTreeMap<String, PackageEntryKind>,
    source: &mut CachedSource<'_>,
) -> Result<(), String> {
    for (relative, kind) in tree {
        if matches!(kind, PackageEntryKind::Regular { .. })
            && (relative.starts_with("validator/src/") || relative.starts_with("validator/tests/"))
            && relative.ends_with(".rs")
        {
            source.read(relative, anchored::MAX_RESOURCE_BYTES)?;
        }
    }
    Ok(())
}

fn validate_manifest_collections(manifest: &serde_json::Value) -> Result<(), String> {
    let mut path_count = 1_usize;
    for key in ["skills", "agents"] {
        let rows = manifest
            .get(key)
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "package snapshot manifest collection is missing".to_string())?;
        path_count = path_count.saturating_add(rows.len());
        if rows.iter().any(|row| {
            row.get("path")
                .and_then(serde_json::Value::as_str)
                .is_none()
        }) {
            return Err("package snapshot manifest component path is invalid".to_string());
        }
    }
    for key in [
        "schemas",
        "fixtures",
        "authorable_templates",
        "generated_examples",
        "resources",
    ] {
        let rows = manifest
            .get(key)
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "package snapshot manifest collection is missing".to_string())?;
        path_count = path_count.saturating_add(rows.len());
        if rows.iter().any(|row| !row.is_string()) {
            return Err("package snapshot manifest collection path is invalid".to_string());
        }
    }
    if manifest
        .get("schema_catalog")
        .and_then(serde_json::Value::as_str)
        .is_none()
    {
        return Err("package snapshot manifest schema catalog is missing".to_string());
    }
    if path_count > MAX_MANIFEST_PATHS {
        return Err("package snapshot manifest exceeds its path limit".to_string());
    }
    Ok(())
}
