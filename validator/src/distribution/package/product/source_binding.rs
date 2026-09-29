#[derive(Debug, Eq, PartialEq)]
pub(super) struct PackageSourceBinding {
    pub(super) context_id: String,
    pub(super) catalog_id: String,
}

#[derive(Serialize)]
struct PackageSourceCatalog<'a> {
    schema: &'static str,
    candidate_id: &'a str,
    plugin_id: &'static str,
    version: &'a str,
    entries: Vec<PackageSourceCatalogEntry<'a>>,
}

#[derive(Serialize)]
struct PackageSourceCatalogEntry<'a> {
    path: &'a str,
    mode: u32,
    role: PackageRole,
    sha256: &'a str,
    byte_length: u64,
}

#[derive(Serialize)]
struct PackageSourceContext<'a> {
    schema: &'static str,
    candidate_id: &'a str,
    plugin_id: &'static str,
    version: &'a str,
    catalog_id: &'a str,
}

/// Derive the durable package identity only from source-owned bytes and the
/// exact Git candidate. The surrounding `LiveContext` still guards each
/// capture and effect, but its PATH, permissions, and operation scope must not
/// make an unchanged archive unverifiable in a later process.
pub(super) fn package_source_binding(
    candidate_id: &str,
    version: &str,
    entries: &[PackageEntry],
) -> Result<PackageSourceBinding, ProductionPackageError> {
    let rows = entries
        .iter()
        .map(|entry| PackageSourceCatalogEntry {
            path: &entry.path,
            mode: entry.mode,
            role: entry.role,
            sha256: &entry.sha256,
            byte_length: entry.bytes.len() as u64,
        })
        .collect();
    let catalog_id = serde_json::to_vec(&PackageSourceCatalog {
        schema: "HarnessPackageSourceCatalog-v1",
        candidate_id,
        plugin_id: PLUGIN_ID,
        version,
        entries: rows,
    })
    .map(|bytes| sha256(&bytes))
    .map_err(|_| failure(ProductionPackageErrorId::InventoryMismatch))?;
    let context_id = serde_json::to_vec(&PackageSourceContext {
        schema: "HarnessPackageSourceContext-v1",
        candidate_id,
        plugin_id: PLUGIN_ID,
        version,
        catalog_id: &catalog_id,
    })
    .map(|bytes| sha256(&bytes))
    .map_err(|_| failure(ProductionPackageErrorId::InventoryMismatch))?;
    Ok(PackageSourceBinding {
        context_id,
        catalog_id,
    })
}
