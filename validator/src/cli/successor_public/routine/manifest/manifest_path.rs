use super::*;

pub(crate) const MANIFEST_PATH: &str = "config/routine-public.json";
pub(crate) const MANIFEST_SCHEMA: &str = "RoutinePublicProduction-v1";
pub(crate) const COMMAND_NAME: &str = "check-routine";
pub(crate) const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
pub(crate) const MAX_NODES: usize = 4_096;
pub(crate) const MAX_ROUTES: usize = 16_384;
pub(crate) const MAX_READ_SOURCES_PER_NODE: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ManifestFailure {
    MissingOrUnreadable,
    Invalid,
    ConcurrentMutation,
}

#[derive(Clone, Debug)]
pub(crate) struct LoadedManifest {
    pub(crate) source_sha256: String,
    pub(crate) source_byte_length: u64,
    pub(crate) catalog: CatalogBinding,
    pub(crate) nodes: Vec<ManifestNode>,
    pub(crate) routes: Vec<ManifestRoute>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CatalogBinding {
    pub(crate) path: String,
    pub(crate) sha256: String,
    pub(crate) byte_length: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RoutineAction {
    Pass,
    Fail,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManifestNode {
    pub(crate) node_id: String,
    pub(crate) depends_on: Vec<String>,
    pub(crate) primary_tool: String,
    pub(crate) fallback_tool: Option<String>,
    pub(crate) action: RoutineAction,
    pub(crate) delay_seconds: u64,
    pub(crate) read_sources: Vec<String>,
    pub(crate) timeout_ms: u64,
    pub(crate) output_budget_bytes: u64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MatcherKind {
    Exact,
    Prefix,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManifestRoute {
    pub(crate) row_id: String,
    pub(crate) matcher: MatcherKind,
    pub(crate) path: String,
    pub(crate) node_ids: Vec<String>,
    pub(crate) requires_strict: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawManifest {
    pub(crate) schema_version: String,
    pub(crate) command: String,
    pub(crate) catalog: CatalogBinding,
    pub(crate) nodes: Vec<ManifestNode>,
    pub(crate) routes: Vec<ManifestRoute>,
}

pub(crate) fn load(
    context: &LiveContext,
    target: &Path,
) -> Result<LoadedManifest, ManifestFailure> {
    let path = target.join(MANIFEST_PATH);
    let reads = context
        .begin_read_session()
        .map_err(|_| ManifestFailure::MissingOrUnreadable)?;
    let bytes = reads
        .read_bounded(&path, MAX_MANIFEST_BYTES)
        .map_err(|_| ManifestFailure::MissingOrUnreadable)?;
    reads
        .revalidate()
        .map_err(|_| ManifestFailure::ConcurrentMutation)?;
    let raw: RawManifest = serde_json::from_slice(&bytes).map_err(|_| ManifestFailure::Invalid)?;
    validate_raw(raw, &bytes)
}

pub(crate) fn validate_raw(
    raw: RawManifest,
    bytes: &[u8],
) -> Result<LoadedManifest, ManifestFailure> {
    if raw.schema_version != MANIFEST_SCHEMA
        || raw.command != COMMAND_NAME
        || raw.nodes.is_empty()
        || raw.nodes.len() > MAX_NODES
        || raw.routes.is_empty()
        || raw.routes.len() > MAX_ROUTES
        || !valid_sha256(&raw.catalog.sha256)
        || raw.catalog.byte_length == 0
        || raw.catalog.byte_length > 1024 * 1024
        || RepoPath::parse(&raw.catalog.path).is_err()
        || raw.catalog.path == MANIFEST_PATH
    {
        return Err(ManifestFailure::Invalid);
    }

    let mut node_ids = BTreeSet::new();
    let mut node_case_ids = BTreeSet::new();
    let mut nodes = raw.nodes;
    for node in &mut nodes {
        if !node_ids.insert(node.node_id.clone())
            || !node_case_ids.insert(node.node_id.to_ascii_lowercase())
            || !allowed_tool(&node.primary_tool)
            || node
                .fallback_tool
                .as_deref()
                .is_some_and(|tool| !allowed_tool(tool) || tool == node.primary_tool)
            || node.delay_seconds > 5
            || !(1..=300_000).contains(&node.timeout_ms)
            || !(1..=16 * 1024 * 1024).contains(&node.output_budget_bytes)
            || node.read_sources.is_empty()
            || node.read_sources.len() > MAX_READ_SOURCES_PER_NODE
        {
            return Err(ManifestFailure::Invalid);
        }
        node.depends_on.sort();
        if has_duplicate(&node.depends_on) || node.depends_on.contains(&node.node_id) {
            return Err(ManifestFailure::Invalid);
        }
        node.read_sources.sort();
        if has_duplicate(&node.read_sources)
            || node
                .read_sources
                .iter()
                .any(|path| RepoPath::parse(path).is_err())
        {
            return Err(ManifestFailure::Invalid);
        }
    }
    nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    if nodes
        .iter()
        .flat_map(|node| node.depends_on.iter())
        .any(|dependency| !node_ids.contains(dependency))
    {
        return Err(ManifestFailure::Invalid);
    }

    let loaded = LoadedManifest {
        source_sha256: digest(bytes),
        source_byte_length: bytes.len() as u64,
        catalog: raw.catalog,
        nodes,
        routes: raw.routes,
    };
    loaded.graph().map_err(|_| ManifestFailure::Invalid)?;
    Ok(loaded)
}
