use super::*;
use crate::package::inventory::{
    DraftPackageManifest, package_digest_excluded, package_path_syntax_error, payload,
};

pub(super) fn read_package(
    tree: &BTreeMap<String, PackageEntryKind>,
    source: &mut CachedSource<'_>,
) -> Result<PackageRead, String> {
    let manifest_bytes = source.read(MANIFEST_PATH, anchored::MAX_MANIFEST_BYTES)?;
    let manifest = DraftPackageManifest::parse(manifest_bytes.as_ref())?;
    let mut listed_paths = manifest
        .inventory_paths()
        .into_iter()
        .filter(|path| !package_digest_excluded(path))
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
        .any(|path| package_path_syntax_error(path).is_some())
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
                Classification::AdoptedSchemaContract => rows.push((
                    relative.clone(),
                    source.read(relative, anchored::MAX_RESOURCE_BYTES)?,
                )),
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

pub(super) fn capture_supported_package_paths(
    tree: &BTreeMap<String, PackageEntryKind>,
    manifest: &DraftPackageManifest,
    source: &mut CachedSource<'_>,
) -> Result<Vec<String>, String> {
    let supported_manifest = ".codex-plugin/plugin.json";
    let runtime_probe = "runtime/runtime-probe-bin";
    if !tree
        .get(supported_manifest)
        .is_some_and(|kind| matches!(kind, PackageEntryKind::Regular { single_link: true }))
    {
        return Err("package snapshot supported manifest is unavailable".to_string());
    }
    source.read(supported_manifest, anchored::MAX_MANIFEST_BYTES)?;

    let skills = manifest.skills();
    let mut roots = Vec::with_capacity(skills.len());
    for row in skills {
        let path = row.path();
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

    let mut required = BTreeSet::from([
        supported_manifest.to_string(),
        runtime_probe.to_string(),
        MARKETPLACE_CATALOG_PATH.to_string(),
    ]);
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
    match tree.get(runtime_probe) {
        Some(PackageEntryKind::Regular { single_link: true }) => {
            source.read(runtime_probe, anchored::MAX_RESOURCE_BYTES)?;
            packaged.push(runtime_probe.to_string());
        }
        _ => return Err("package snapshot runtime probe is unavailable or unsafe".to_string()),
    }
    match tree.get(MARKETPLACE_CATALOG_PATH) {
        Some(PackageEntryKind::Regular { single_link: true }) => {
            source.read(MARKETPLACE_CATALOG_PATH, anchored::MAX_RESOURCE_BYTES)?;
            packaged.push(MARKETPLACE_CATALOG_PATH.to_string());
        }
        _ => {
            return Err(
                "package snapshot marketplace catalog is unavailable or unsafe".to_string(),
            );
        }
    }
    if tree
        .keys()
        .any(|path| path.starts_with("runtime/") && path != runtime_probe)
    {
        return Err("package snapshot runtime subtree contains an unknown member".to_string());
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

pub(in super::super) fn supported_skill_root<'a>(
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

pub(super) fn digest_rows(rows: &[(String, Arc<[u8]>)]) -> Result<String, String> {
    let mut hasher = Sha256::new();
    for (relative, bytes) in rows {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(payload::stable_package_payload(relative, bytes.as_ref())?);
        hasher.update([0]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

pub(super) fn cache_rust_sources(
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
