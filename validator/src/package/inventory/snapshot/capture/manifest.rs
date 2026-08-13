use super::*;
use crate::agent_roles::CANONICAL_AGENT_ROLES;
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

    let agent_paths = declared_agent_paths(manifest)?;

    let mut required = BTreeSet::from([
        supported_manifest.to_string(),
        runtime_probe.to_string(),
        MARKETPLACE_CATALOG_PATH.to_string(),
    ]);
    for root in &roots {
        required.insert(format!("{root}/SKILL.md"));
        required.insert(format!("{root}/agents/openai.yaml"));
    }
    required.extend(agent_paths.iter().cloned());

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
    for (path, kind) in tree {
        if !supported_agent_member(path, &agent_paths)? {
            continue;
        }
        match kind {
            PackageEntryKind::Regular { single_link: true } => {
                source.read(path, anchored::MAX_RESOURCE_BYTES)?;
                packaged.push(path.clone());
            }
            PackageEntryKind::Directory
            | PackageEntryKind::Regular { single_link: false }
            | PackageEntryKind::Symlink
            | PackageEntryKind::Special => {
                return Err("package snapshot agent member is unsafe".to_string());
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

fn declared_agent_paths(manifest: &DraftPackageManifest) -> Result<Vec<String>, String> {
    let agents = manifest.agents();
    if agents.is_empty() {
        return Ok(Vec::new());
    }
    let canonical = CANONICAL_AGENT_ROLES
        .iter()
        .map(|role| (role.name, role.manifest_path))
        .collect::<BTreeSet<_>>();
    let declared = agents
        .iter()
        .map(|agent| (agent.name(), agent.path()))
        .collect::<BTreeSet<_>>();
    if agents.len() != CANONICAL_AGENT_ROLES.len()
        || declared.len() != agents.len()
        || declared != canonical
    {
        return Err("package snapshot manifest agent declarations do not match".to_string());
    }
    Ok(canonical
        .into_iter()
        .map(|(_, path)| path.to_owned())
        .collect())
}

fn supported_agent_member(path: &str, declared: &[String]) -> Result<bool, String> {
    const ROOT: &str = ".codex/agents";
    let folded = path.to_ascii_lowercase();
    if folded == ROOT {
        if path != ROOT {
            return Err("package snapshot agent subtree has a case collision".to_string());
        }
        return Ok(false);
    }
    if !folded.starts_with(".codex/agents/") {
        return Ok(false);
    }
    if !path.starts_with(".codex/agents/")
        || declared
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(path) && candidate != path)
    {
        return Err("package snapshot agent subtree has a case collision".to_string());
    }
    if !declared.iter().any(|candidate| candidate == path) {
        return Err("package snapshot agent subtree contains an unknown member".to_string());
    }
    Ok(true)
}

#[cfg(test)]
mod agent_member_tests {
    use super::*;

    fn declared() -> Vec<String> {
        CANONICAL_AGENT_ROLES
            .iter()
            .map(|role| role.manifest_path.to_owned())
            .collect()
    }

    fn manifest(agents: serde_json::Value, resources: serde_json::Value) -> DraftPackageManifest {
        parse_manifest(agents, resources).expect("draft manifest")
    }

    fn parse_manifest(
        agents: serde_json::Value,
        resources: serde_json::Value,
    ) -> Result<DraftPackageManifest, String> {
        DraftPackageManifest::parse(
            &serde_json::to_vec(&serde_json::json!({
                "name": "snapshot-test",
                "version": "0.0.0",
                "status": "test",
                "purpose": "test",
                "skills": [],
                "agents": agents,
                "schemas": [],
                "fixtures": [],
                "authorable_templates": [],
                "generated_examples": [],
                "resources": resources,
                "schema_catalog": "schemas/catalog.json",
                "optional_connectors": [],
                "non_goals": []
            }))
            .expect("manifest JSON"),
        )
    }

    fn canonical_agents() -> serde_json::Value {
        serde_json::Value::Array(
            CANONICAL_AGENT_ROLES
                .iter()
                .map(|role| {
                    serde_json::json!({
                        "name": role.name,
                        "path": role.manifest_path
                    })
                })
                .collect(),
        )
    }

    #[test]
    fn declarations_allow_empty_or_exact_unordered_canonical_pair_sets() {
        let generic = manifest(serde_json::json!([]), serde_json::json!([]));
        assert_eq!(declared_agent_paths(&generic), Ok(Vec::new()));

        let canonical = manifest(canonical_agents(), serde_json::json!([]));
        assert_eq!(declared_agent_paths(&canonical), Ok(declared()));

        let laundered = manifest(
            serde_json::json!([]),
            serde_json::Value::Array(
                CANONICAL_AGENT_ROLES
                    .iter()
                    .map(|role| serde_json::json!(role.manifest_path))
                    .collect(),
            ),
        );
        let declarations = declared_agent_paths(&laundered).expect("generic declarations");
        assert!(declarations.is_empty());
        assert!(
            supported_agent_member(CANONICAL_AGENT_ROLES[0].manifest_path, &declarations).is_err()
        );

        let mut mismatched = canonical_agents();
        mismatched[0]["name"] = serde_json::json!(CANONICAL_AGENT_ROLES[1].name);
        mismatched[1]["name"] = serde_json::json!(CANONICAL_AGENT_ROLES[0].name);
        let mismatched = manifest(mismatched, serde_json::json!([]));
        assert!(declared_agent_paths(&mismatched).is_err());

        let mut reordered = canonical_agents();
        reordered.as_array_mut().expect("agent rows").swap(0, 1);
        let reordered = manifest(reordered, serde_json::json!([]));
        assert_eq!(declared_agent_paths(&reordered), Ok(declared()));

        let mut duplicate = canonical_agents();
        duplicate[1] = duplicate[0].clone();
        assert!(parse_manifest(duplicate, serde_json::json!([])).is_err());
    }

    #[test]
    fn canonical_agent_members_are_exact_and_case_collision_safe() {
        let declared = declared();
        assert_eq!(
            supported_agent_member(CANONICAL_AGENT_ROLES[0].manifest_path, &declared),
            Ok(true)
        );
        assert_eq!(
            supported_agent_member(".codex/agents", &declared),
            Ok(false)
        );
        assert!(supported_agent_member(".codex/Agents", &declared).is_err());
        assert!(supported_agent_member(".codex/agents/Claim-Falsifier.toml", &declared).is_err());
        assert!(supported_agent_member(".codex/agents/undeclared.toml", &declared).is_err());
        assert_eq!(
            supported_agent_member("skills/prove/SKILL.md", &declared),
            Ok(false)
        );
    }

    #[test]
    fn only_regular_single_link_agent_members_are_safe() {
        for kind in [
            PackageEntryKind::Directory,
            PackageEntryKind::Regular { single_link: false },
            PackageEntryKind::Symlink,
            PackageEntryKind::Special,
        ] {
            assert!(unsafe_agent_member(kind));
        }
        assert!(!unsafe_agent_member(PackageEntryKind::Regular {
            single_link: true
        }));
    }

    fn unsafe_agent_member(kind: PackageEntryKind) -> bool {
        !matches!(kind, PackageEntryKind::Regular { single_link: true })
    }
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
