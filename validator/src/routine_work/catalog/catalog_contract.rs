use super::*;

pub(crate) const CATALOG_SCHEMA: &str = "RoutineProductionCatalog-v2";
pub(crate) const ROUTINE_BEHAVIOR: &str = "rust-source-syntax-v1";
pub(crate) const ROUTINE_RUNNER: &str = "ultragoal";
pub(crate) const ROUTINE_ARGUMENTS: [&str; 3] = ["--json", "check", "routine"];
pub(crate) const RUNNER_POLICY: &str = "immutable-single-process-exact-executable-v1";
pub(crate) const READ_POLICY: &str = "selected-transitive-exact-regular-files-v1";
pub(crate) const MAX_CATALOG_BYTES: u64 = 1024 * 1024;
pub(crate) const MAX_DEFINITIONS: usize = 4_096;
pub(crate) const MAX_DEPENDENCIES: usize = 1_024;
pub(crate) const MAX_READ_SOURCES: usize = 128;
pub(crate) const MAX_READ_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_OUTPUT_SCOPES: usize = 128;
pub(crate) const MAX_TIMEOUT_MS: u64 = 3_600_000;
pub(crate) const MAX_OUTPUT_BUDGET_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RoutineCatalogError {
    pub(crate) code: &'static str,
}

impl RoutineCatalogError {
    pub(crate) fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub(crate) fn code(&self) -> &'static str {
        self.code
    }
}

impl std::fmt::Display for RoutineCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for RoutineCatalogError {}

pub(crate) type CatalogResult<T> = Result<T, RoutineCatalogError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct AdoptedRoutineNode {
    pub(crate) node_id: String,
    pub(crate) depends_on: BTreeSet<String>,
}

impl AdoptedRoutineNode {
    pub(crate) fn new(
        node_id: impl Into<String>,
        depends_on: impl IntoIterator<Item = String>,
    ) -> CatalogResult<Self> {
        let node_id = identifier(node_id.into())?;
        let depends_on = identifier_set(depends_on, "catalog-adoption-dependency-duplicated")?;
        if depends_on.len() > MAX_DEPENDENCIES || depends_on.contains(&node_id) {
            return Err(error("catalog-adoption-dependencies-invalid"));
        }
        Ok(Self {
            node_id,
            depends_on,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CatalogAdoption {
    pub(crate) source_sha256: String,
    pub(crate) source_byte_length: u64,
    pub(crate) graph_id: String,
    pub(crate) candidate_id: String,
    pub(crate) nodes: BTreeMap<String, AdoptedRoutineNode>,
}

impl CatalogAdoption {
    pub(crate) fn new(
        source_sha256: impl Into<String>,
        source_byte_length: u64,
        graph_id: impl Into<String>,
        candidate_id: impl Into<String>,
        nodes: Vec<AdoptedRoutineNode>,
    ) -> CatalogResult<Self> {
        let source_sha256 = required_sha256(source_sha256.into(), "catalog-source-digest-invalid")?;
        if source_byte_length == 0 || source_byte_length > MAX_CATALOG_BYTES {
            return Err(error("catalog-source-length-invalid"));
        }
        let graph_id = required_sha256(graph_id.into(), "catalog-graph-id-invalid")?;
        let candidate_id = required_sha256(candidate_id.into(), "catalog-candidate-id-invalid")?;
        if nodes.is_empty() || nodes.len() > MAX_DEFINITIONS {
            return Err(error("catalog-adoption-node-count-invalid"));
        }
        let mut by_id = BTreeMap::new();
        let mut case_ids = BTreeSet::new();
        for node in nodes {
            if !case_ids.insert(node.node_id.to_ascii_lowercase()) {
                return Err(error("catalog-adoption-node-ambiguous"));
            }
            if by_id.insert(node.node_id.clone(), node).is_some() {
                return Err(error("catalog-adoption-node-duplicated"));
            }
        }
        if by_id.values().any(|node| {
            node.depends_on
                .iter()
                .any(|dependency| !by_id.contains_key(dependency))
        }) {
            return Err(error("catalog-adoption-dependency-unknown"));
        }
        validate_adoption_dag(&by_id)?;
        Ok(Self {
            source_sha256,
            source_byte_length,
            graph_id,
            candidate_id,
            nodes: by_id,
        })
    }
}

pub(crate) fn validate_adoption_dag(
    nodes: &BTreeMap<String, AdoptedRoutineNode>,
) -> CatalogResult<()> {
    fn visit(
        node_id: &str,
        nodes: &BTreeMap<String, AdoptedRoutineNode>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> CatalogResult<()> {
        if visited.contains(node_id) {
            return Ok(());
        }
        if !visiting.insert(node_id.to_owned()) {
            return Err(error("catalog-adoption-dependency-cycle"));
        }
        for dependency in &nodes[node_id].depends_on {
            visit(dependency, nodes, visiting, visited)?;
        }
        visiting.remove(node_id);
        visited.insert(node_id.to_owned());
        Ok(())
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for node_id in nodes.keys() {
        visit(node_id, nodes, &mut visiting, &mut visited)?;
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TransitiveInputExpectation {
    pub(crate) relative_path: CatalogPath,
    pub(crate) sha256: String,
    pub(crate) byte_length: u64,
}

impl TransitiveInputExpectation {
    pub(crate) fn new(
        relative_path: impl Into<String>,
        sha256: impl Into<String>,
        byte_length: u64,
    ) -> CatalogResult<Self> {
        if byte_length > MAX_READ_SOURCE_BYTES {
            return Err(error("catalog-input-length-invalid"));
        }
        Ok(Self {
            relative_path: CatalogPath::parse(relative_path.into())?,
            sha256: required_sha256(sha256.into(), "catalog-input-digest-invalid")?,
            byte_length,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectedRoutineNode {
    pub(crate) node_id: String,
    pub(crate) depends_on: BTreeSet<String>,
    pub(crate) input_id: String,
    pub(crate) transitive_inputs: Vec<TransitiveInputExpectation>,
}
