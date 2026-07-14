const MAGIC: &[u8; 8] = b"HUGPKG1\0";
const ARCHIVE_OVERHEAD_LIMIT: usize = 1024 * 1024;
const ENTRY_COUNT_LIMIT: usize = 4096;

pub(crate) struct DecodedArchive {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plugin_id: String,
    pub(crate) version: String,
    pub(crate) catalog_id: String,
    pub(crate) accepted_inventory_sha256: String,
    pub(crate) source_tree_sha256: String,
    pub(crate) source_date_epoch: u64,
    pub(crate) entries: Vec<PackageEntry>,
}

pub(crate) fn encode(plan: &PackagePlan) -> Result<Vec<u8>, DistributionError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MAGIC);
    push_string(&mut bytes, &plan.context_id)?;
    push_string(&mut bytes, &plan.candidate_id)?;
    push_string(&mut bytes, &plan.plugin_id)?;
    push_string(&mut bytes, &plan.version)?;
    push_string(&mut bytes, &plan.catalog_id)?;
    push_string(&mut bytes, &plan.accepted_inventory_sha256)?;
    push_string(&mut bytes, &plan.source_tree_sha256)?;
    bytes.extend_from_slice(&plan.source_date_epoch.to_be_bytes());
    bytes.extend_from_slice(&(plan.entries.len() as u32).to_be_bytes());
    for entry in &plan.entries {
        push_string(&mut bytes, &entry.path)?;
        bytes.extend_from_slice(&entry.mode.to_be_bytes());
        bytes.push(entry.role.code());
        bytes.extend_from_slice(&(entry.bytes.len() as u64).to_be_bytes());
        push_string(&mut bytes, &entry.sha256)?;
        bytes.extend_from_slice(&entry.bytes);
    }
    Ok(bytes)
}

pub(crate) fn decode(bytes: &[u8]) -> Result<DecodedArchive, DistributionError> {
    if bytes.len() > PACKAGE_LIMIT + ARCHIVE_OVERHEAD_LIMIT {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    let mut reader = Reader::new(bytes);
    if reader.take(MAGIC.len())? != MAGIC {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    let context_id = reader.string(128)?;
    let candidate_id = reader.string(128)?;
    let plugin_id = reader.string(128)?;
    let version = reader.string(128)?;
    let catalog_id = reader.string(128)?;
    let accepted_inventory_sha256 = reader.string(128)?;
    let source_tree_sha256 = reader.string(128)?;
    if !digest(&context_id)
        || !digest(&candidate_id)
        || plugin_id != "harness-ultragoal"
        || Version::parse(&version).is_none()
        || !digest(&catalog_id)
        || !digest(&accepted_inventory_sha256)
        || !digest(&source_tree_sha256)
    {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    let source_date_epoch = reader.u64()?;
    let entry_count = reader.u32()? as usize;
    if entry_count == 0 {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    if entry_count > ENTRY_COUNT_LIMIT {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    let mut entries = Vec::with_capacity(entry_count);
    let mut targets = BTreeSet::new();
    let mut prior: Option<String> = None;
    let mut total = 0usize;
    for _ in 0..entry_count {
        let path = reader.string(512)?;
        validate_relative_path(&path)?;
        let mode = reader.u32()?;
        let role = decode_role(reader.u8()?)?;
        let byte_length = usize::try_from(reader.u64()?)
            .map_err(|_| error(DistributionErrorId::ObjectTooLarge))?;
        let declared_sha256 = reader.string(128)?;
        if byte_length > ENTRY_LIMIT {
            return Err(error(DistributionErrorId::ObjectTooLarge));
        }
        if prior.as_deref().is_some_and(|value| value >= path.as_str())
            || !insert_prefix_free_path(&mut targets, &path)
            || !matches!(mode, 0o644 | 0o755)
            || (role == PackageRole::Executable) != (mode == 0o755)
            || (path == ".codex-plugin/plugin.json") != (role == PackageRole::Manifest)
            || !digest(&declared_sha256)
        {
            return Err(error(DistributionErrorId::ArchiveMismatch));
        }
        total = total
            .checked_add(byte_length)
            .ok_or_else(|| error(DistributionErrorId::ObjectTooLarge))?;
        if total > PACKAGE_LIMIT {
            return Err(error(DistributionErrorId::ObjectTooLarge));
        }
        let entry_bytes = reader.take(byte_length)?.to_vec();
        if sha256(&entry_bytes) != declared_sha256 {
            return Err(error(DistributionErrorId::ArchiveMismatch));
        }
        prior = Some(path.clone());
        entries.push(PackageEntry {
            path,
            mode,
            role,
            sha256: declared_sha256,
            bytes: entry_bytes,
        });
    }
    if !reader.finished() || !targets.contains(".codex-plugin/plugin.json") {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    Ok(DecodedArchive {
        context_id,
        candidate_id,
        plugin_id,
        version,
        catalog_id,
        accepted_inventory_sha256,
        source_tree_sha256,
        source_date_epoch,
        entries,
    })
}

pub(crate) fn verify_plan(
    archive: &DecodedArchive,
    plan: &PackagePlan,
) -> Result<(), DistributionError> {
    if archive.context_id != plan.context_id
        || archive.candidate_id != plan.candidate_id
        || archive.plugin_id != plan.plugin_id
        || archive.version != plan.version
        || archive.catalog_id != plan.catalog_id
        || archive.accepted_inventory_sha256 != plan.accepted_inventory_sha256
        || archive.source_tree_sha256 != plan.source_tree_sha256
        || archive.source_date_epoch != plan.source_date_epoch
        || archive.entries != plan.entries
    {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    Ok(())
}
