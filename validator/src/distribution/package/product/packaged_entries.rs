fn packaged_entries(
    source: &SourcePackageSnapshot,
) -> Result<Vec<PackageEntry>, ProductionPackageError> {
    let mut entries = Vec::with_capacity(source.packaged_paths().len());
    for path in source.packaged_paths() {
        let bytes = source
            .bytes(path)
            .ok_or_else(|| failure(ProductionPackageErrorId::MembershipMismatch))?;
        let mode = source
            .unix_mode(path)
            .ok_or_else(|| failure(ProductionPackageErrorId::MembershipMismatch))?;
        let role = package_role(path, mode)?;
        entries.push(PackageEntry {
            path: path.clone(),
            mode,
            role,
            sha256: sha256(bytes),
            bytes: bytes.to_vec(),
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    if entries.len() != source.packaged_paths().len()
        || entries.windows(2).any(|pair| pair[0].path == pair[1].path)
        || entries
            .iter()
            .filter(|entry| entry.role == PackageRole::Manifest)
            .count()
            != 1
    {
        return Err(failure(ProductionPackageErrorId::MembershipMismatch));
    }
    for name in CANONICAL_SKILLS {
        for required in [
            format!("skills/{name}/SKILL.md"),
            format!("skills/{name}/agents/openai.yaml"),
        ] {
            if !entries.iter().any(|entry| entry.path == required) {
                return Err(failure(ProductionPackageErrorId::MembershipMismatch));
            }
        }
    }
    Ok(entries)
}

fn package_role(path: &str, mode: u32) -> Result<PackageRole, ProductionPackageError> {
    let role = if path == SUPPORTED_MANIFEST_PATH {
        PackageRole::Manifest
    } else if path == "runtime/runtime-probe-bin" {
        PackageRole::Executable
    } else if CANONICAL_SKILLS
        .iter()
        .any(|name| path == format!("skills/{name}/SKILL.md"))
    {
        PackageRole::Skill
    } else if CANONICAL_SKILLS
        .iter()
        .any(|name| path == format!("skills/{name}/agents/openai.yaml"))
    {
        PackageRole::Agent
    } else {
        return Err(failure(ProductionPackageErrorId::MembershipMismatch));
    };
    if (role == PackageRole::Executable && mode != 0o755)
        || (role != PackageRole::Executable && mode != 0o644)
    {
        return Err(failure(ProductionPackageErrorId::MembershipMismatch));
    }
    Ok(role)
}

#[derive(Serialize)]
struct SourceInventory<'a> {
    schema: &'static str,
    context_id: &'a str,
    candidate_id: &'a str,
    catalog_id: &'a str,
    plugin_id: &'static str,
    version: &'a str,
    source_date_epoch: u64,
    entries: Vec<SourceInventoryEntry<'a>>,
}

#[derive(Serialize)]
struct SourceInventoryEntry<'a> {
    path: &'a str,
    object_type: &'static str,
    mode: u32,
    sha256: &'a str,
    byte_length: u64,
    role: PackageRole,
}

fn source_inventory(
    context_id: &str,
    candidate_id: &str,
    catalog_id: &str,
    version: &str,
    entries: &[PackageEntry],
) -> Result<Vec<u8>, ProductionPackageError> {
    let rows = entries
        .iter()
        .map(|entry| SourceInventoryEntry {
            path: &entry.path,
            object_type: "regular-file",
            mode: entry.mode,
            sha256: &entry.sha256,
            byte_length: entry.bytes.len() as u64,
            role: entry.role,
        })
        .collect();
    serde_json::to_vec(&SourceInventory {
        schema: "harness-ultragoal.accepted-package-source-set.v1",
        context_id,
        candidate_id,
        catalog_id,
        plugin_id: PLUGIN_ID,
        version,
        source_date_epoch: 0,
        entries: rows,
    })
    .map_err(|_| failure(ProductionPackageErrorId::InventoryMismatch))
}

fn candidate_id(context: &LiveContext) -> Result<String, ProductionPackageError> {
    serde_json::to_vec(context.candidate())
        .map(|bytes| sha256(&bytes))
        .map_err(|_| failure(ProductionPackageErrorId::ContextUnavailable))
}
