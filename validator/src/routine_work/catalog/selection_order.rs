use super::*;

pub(crate) fn canonical_selected_order(
    definitions: &BTreeMap<String, RoutineDefinition>,
    selected: &BTreeSet<String>,
) -> CatalogResult<Vec<String>> {
    if selected.iter().any(|node| !definitions.contains_key(node)) {
        return Err(error("catalog-selection-node-unknown"));
    }
    let mut remaining = selected.clone();
    let mut emitted = BTreeSet::new();
    let mut order = Vec::with_capacity(selected.len());
    while !remaining.is_empty() {
        let ready = remaining
            .iter()
            .filter(|node| {
                definitions[*node].depends_on.iter().all(|dependency| {
                    !selected.contains(dependency) || emitted.contains(dependency)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if ready.is_empty() {
            return Err(error("catalog-selection-dependency-cycle"));
        }
        for node in ready {
            remaining.remove(&node);
            emitted.insert(node.clone());
            order.push(node);
        }
    }
    Ok(order)
}

pub(crate) fn load_production_catalog(
    worktree_root: &Path,
    relative_path: &Path,
    adoption: CatalogAdoption,
) -> CatalogResult<ProductionRoutineCatalog> {
    let relative_text = relative_path
        .to_str()
        .ok_or_else(|| error("catalog-source-path-not-utf8"))?;
    let relative = CatalogPath::parse(relative_text.to_owned())?;
    let source = SealedFile::capture_relative(worktree_root, &relative, MAX_CATALOG_BYTES)?;
    if source.identity.sha256 != adoption.source_sha256
        || source.identity.byte_length != adoption.source_byte_length
    {
        return Err(error("catalog-source-adoption-mismatch"));
    }
    let text = std::str::from_utf8(&source.bytes).map_err(|_| error("catalog-source-not-utf8"))?;
    let raw: RawCatalog =
        serde_json::from_str(text).map_err(|_| error("catalog-source-invalid-json"))?;
    let definitions = validate_raw_catalog(raw, &adoption)?;
    let catalog_id = digest_json(&CatalogIdentity {
        source_sha256: &source.identity.sha256,
        source_byte_length: source.identity.byte_length,
        graph_id: &adoption.graph_id,
        candidate_id: &adoption.candidate_id,
        definitions: &definitions,
    })?;
    source.verify_current(MAX_CATALOG_BYTES)?;
    Ok(ProductionRoutineCatalog {
        source,
        catalog_id,
        graph_id: adoption.graph_id,
        candidate_id: adoption.candidate_id,
        definitions,
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawCatalog {
    pub(crate) schema_version: String,
    pub(crate) graph_id: String,
    pub(crate) routines: Vec<RawRoutineDefinition>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawRoutineDefinition {
    pub(crate) node_id: String,
    pub(crate) behavior_id: String,
    pub(crate) depends_on: Vec<String>,
    pub(crate) read_sources: Vec<String>,
    pub(crate) timeout_ms: u64,
    pub(crate) output_budget_bytes: u64,
    pub(crate) output_scopes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RoutineDefinition {
    pub(crate) definition_id: String,
    pub(crate) definition_sha256: String,
    pub(crate) node_id: String,
    pub(crate) behavior_id: String,
    pub(crate) depends_on: BTreeSet<String>,
    pub(crate) read_sources: Vec<CatalogPath>,
    pub(crate) timeout_ms: u64,
    pub(crate) output_budget_bytes: u64,
    pub(crate) output_scopes: Vec<CatalogPath>,
}
