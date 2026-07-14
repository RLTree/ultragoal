use super::*;

impl SelectedRoutineNode {
    pub(crate) fn new(
        node_id: impl Into<String>,
        depends_on: impl IntoIterator<Item = String>,
        input_id: impl Into<String>,
        mut transitive_inputs: Vec<TransitiveInputExpectation>,
    ) -> CatalogResult<Self> {
        let node_id = identifier(node_id.into())?;
        let depends_on = identifier_set(depends_on, "catalog-selection-dependency-duplicated")?;
        let input_id = required_sha256(input_id.into(), "catalog-selection-input-id-invalid")?;
        normalize_input_expectations(&mut transitive_inputs)?;
        Ok(Self {
            node_id,
            depends_on,
            input_id,
            transitive_inputs,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RunnerObservation {
    pub(crate) tool_name: String,
    pub(crate) tool_identity_sha256: String,
    pub(crate) executable_path: PathBuf,
    pub(crate) program_sha256: String,
    pub(crate) program_byte_length: u64,
    pub(crate) program_unix_mode: u32,
}

impl RunnerObservation {
    pub(crate) fn new(
        tool_name: impl Into<String>,
        tool_identity_sha256: impl Into<String>,
        executable_path: impl Into<PathBuf>,
        program_sha256: impl Into<String>,
        program_byte_length: u64,
        program_unix_mode: u32,
    ) -> CatalogResult<Self> {
        let executable_path = executable_path.into();
        if !executable_path.is_absolute() || executable_path.to_str().is_none() {
            return Err(error("catalog-runner-path-invalid"));
        }
        Ok(Self {
            tool_name: identifier(tool_name.into())?,
            tool_identity_sha256: required_sha256(
                tool_identity_sha256.into(),
                "catalog-runner-tool-identity-invalid",
            )?,
            executable_path,
            program_sha256: required_sha256(
                program_sha256.into(),
                "catalog-runner-program-digest-invalid",
            )?,
            program_byte_length,
            program_unix_mode,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CatalogSelectionRequest {
    pub(crate) catalog_id: String,
    pub(crate) graph_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plan_id: String,
    pub(crate) selected: Vec<SelectedRoutineNode>,
    pub(crate) runners: BTreeMap<String, RunnerObservation>,
}

impl CatalogSelectionRequest {
    pub(crate) fn new(
        catalog_id: impl Into<String>,
        graph_id: impl Into<String>,
        candidate_id: impl Into<String>,
        plan_id: impl Into<String>,
        selected: Vec<SelectedRoutineNode>,
        runners: Vec<RunnerObservation>,
    ) -> CatalogResult<Self> {
        if selected.is_empty() || selected.len() > MAX_DEFINITIONS {
            return Err(error("catalog-selection-cardinality-invalid"));
        }
        let mut selected_ids = BTreeSet::new();
        let mut selected_case_ids = BTreeSet::new();
        let mut dependency_closure = BTreeSet::new();
        for row in &selected {
            if !selected_ids.insert(row.node_id.clone()) {
                return Err(error("catalog-selection-node-duplicated"));
            }
            if !selected_case_ids.insert(row.node_id.to_ascii_lowercase()) {
                return Err(error("catalog-selection-node-ambiguous"));
            }
            if !row.depends_on.is_subset(&dependency_closure) {
                return Err(error("catalog-selection-dependency-closure-incomplete"));
            }
            dependency_closure.insert(row.node_id.clone());
        }
        let mut runner_map = BTreeMap::new();
        let mut runner_case_ids = BTreeSet::new();
        for runner in runners {
            if !runner_case_ids.insert(runner.tool_name.to_ascii_lowercase()) {
                return Err(error("catalog-runner-observation-ambiguous"));
            }
            if runner_map
                .insert(runner.tool_name.clone(), runner)
                .is_some()
            {
                return Err(error("catalog-runner-observation-duplicated"));
            }
        }
        if runner_map.len() != 1 || !runner_map.contains_key(ROUTINE_RUNNER) {
            return Err(error("catalog-runner-observation-set-inexact"));
        }
        Ok(Self {
            catalog_id: required_sha256(catalog_id.into(), "catalog-request-id-invalid")?,
            graph_id: required_sha256(graph_id.into(), "catalog-request-graph-id-invalid")?,
            candidate_id: required_sha256(
                candidate_id.into(),
                "catalog-request-candidate-id-invalid",
            )?,
            plan_id: required_sha256(plan_id.into(), "catalog-request-plan-id-invalid")?,
            selected,
            runners: runner_map,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct BoundReadSource {
    pub(crate) relative_path: String,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) unix_mode: u32,
    pub(crate) owner_user_id: u32,
    pub(crate) owner_group_id: u32,
    pub(crate) link_count: u64,
    pub(crate) byte_length: u64,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanos: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanos: i64,
    pub(crate) sha256: String,
    pub(crate) ancestors: Vec<DirectoryIdentity>,
}

impl BoundReadSource {
    pub(crate) fn relative_path(&self) -> &str {
        &self.relative_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct BoundOutputScope {
    pub(crate) relative_path: String,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) unix_mode: u32,
    pub(crate) owner_user_id: u32,
    pub(crate) owner_group_id: u32,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanos: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanos: i64,
    pub(crate) ancestors: Vec<DirectoryIdentity>,
}

impl BoundOutputScope {
    pub(crate) fn relative_path(&self) -> &str {
        &self.relative_path
    }
}
