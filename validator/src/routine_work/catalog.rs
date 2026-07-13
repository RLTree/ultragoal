//! Bounded production routine definitions and immutable invocation identities.
//!
//! This module deliberately stops before effect authority. It loads one adopted
//! catalog without following links, matches every definition to an independently
//! supplied impact-graph projection, joins selected plan rows to exact transitive
//! inputs and runner observations, and emits immutable invocation descriptions.
//! It never spawns a process, creates an output directory, writes a receipt, or
//! issues a root grant.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

const CATALOG_SCHEMA: &str = "RoutineProductionCatalog-v1";
const RUNNER_POLICY: &str = "immutable-single-process-exact-executable-v1";
const READ_POLICY: &str = "selected-transitive-exact-regular-files-v1";
const FALLBACK_EQUIVALENCE: &str = "same-node-semantics-v1";
const MAX_CATALOG_BYTES: u64 = 1024 * 1024;
const MAX_DEFINITIONS: usize = 4_096;
const MAX_DEPENDENCIES: usize = 1_024;
const MAX_ARGUMENTS: usize = 128;
const MAX_ARGUMENT_BYTES: usize = 64 * 1024;
const MAX_ENVIRONMENT_ENTRIES: usize = 61;
const MAX_ENVIRONMENT_BYTES: usize = 60 * 1024;
const MAX_READ_SOURCES: usize = 128;
const MAX_READ_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_OUTPUT_SCOPES: usize = 128;
const MAX_TIMEOUT_MS: u64 = 3_600_000;
const MAX_OUTPUT_BUDGET_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RoutineCatalogError {
    code: &'static str,
}

impl RoutineCatalogError {
    fn new(code: &'static str) -> Self {
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

type CatalogResult<T> = Result<T, RoutineCatalogError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct AdoptedRoutineNode {
    node_id: String,
    depends_on: BTreeSet<String>,
    primary_tool: String,
    fallback_tool: Option<String>,
}

impl AdoptedRoutineNode {
    pub(crate) fn new(
        node_id: impl Into<String>,
        depends_on: impl IntoIterator<Item = String>,
        primary_tool: impl Into<String>,
        fallback_tool: Option<String>,
    ) -> CatalogResult<Self> {
        let node_id = identifier(node_id.into())?;
        let depends_on = identifier_set(depends_on, "catalog-adoption-dependency-duplicated")?;
        if depends_on.len() > MAX_DEPENDENCIES || depends_on.contains(&node_id) {
            return Err(error("catalog-adoption-dependencies-invalid"));
        }
        let primary_tool = identifier(primary_tool.into())?;
        let fallback_tool = fallback_tool.map(identifier).transpose()?;
        if fallback_tool
            .as_ref()
            .is_some_and(|fallback| fallback.eq_ignore_ascii_case(&primary_tool))
        {
            return Err(error("catalog-adoption-runner-duplicated"));
        }
        Ok(Self {
            node_id,
            depends_on,
            primary_tool,
            fallback_tool,
        })
    }

    pub(crate) fn node_id(&self) -> &str {
        &self.node_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CatalogAdoption {
    source_sha256: String,
    source_byte_length: u64,
    graph_id: String,
    candidate_id: String,
    nodes: BTreeMap<String, AdoptedRoutineNode>,
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

fn validate_adoption_dag(nodes: &BTreeMap<String, AdoptedRoutineNode>) -> CatalogResult<()> {
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
    relative_path: CatalogPath,
    sha256: String,
    byte_length: u64,
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

    pub(crate) fn relative_path(&self) -> &str {
        self.relative_path.as_str()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectedRoutineNode {
    node_id: String,
    depends_on: BTreeSet<String>,
    selected_tool: String,
    selected_tool_identity_sha256: String,
    used_fallback: bool,
    input_id: String,
    transitive_inputs: Vec<TransitiveInputExpectation>,
}

impl SelectedRoutineNode {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        node_id: impl Into<String>,
        depends_on: impl IntoIterator<Item = String>,
        selected_tool: impl Into<String>,
        selected_tool_identity_sha256: impl Into<String>,
        used_fallback: bool,
        input_id: impl Into<String>,
        mut transitive_inputs: Vec<TransitiveInputExpectation>,
    ) -> CatalogResult<Self> {
        let node_id = identifier(node_id.into())?;
        let depends_on = identifier_set(depends_on, "catalog-selection-dependency-duplicated")?;
        let selected_tool = identifier(selected_tool.into())?;
        let selected_tool_identity_sha256 = required_sha256(
            selected_tool_identity_sha256.into(),
            "catalog-selection-tool-identity-invalid",
        )?;
        let input_id = required_sha256(input_id.into(), "catalog-selection-input-id-invalid")?;
        normalize_input_expectations(&mut transitive_inputs)?;
        Ok(Self {
            node_id,
            depends_on,
            selected_tool,
            selected_tool_identity_sha256,
            used_fallback,
            input_id,
            transitive_inputs,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RunnerObservation {
    tool_name: String,
    tool_identity_sha256: String,
    executable_path: PathBuf,
    program_sha256: String,
    program_byte_length: u64,
    program_unix_mode: u32,
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
    catalog_id: String,
    graph_id: String,
    candidate_id: String,
    plan_id: String,
    selected: Vec<SelectedRoutineNode>,
    runners: BTreeMap<String, RunnerObservation>,
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
        let selected_tools = selected
            .iter()
            .map(|row| row.selected_tool.as_str())
            .collect::<BTreeSet<_>>();
        if runner_map
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != selected_tools
        {
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
    relative_path: String,
    device: u64,
    inode: u64,
    unix_mode: u32,
    owner_user_id: u32,
    owner_group_id: u32,
    link_count: u64,
    byte_length: u64,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
    sha256: String,
    ancestors: Vec<DirectoryIdentity>,
}

impl BoundReadSource {
    pub(crate) fn relative_path(&self) -> &str {
        &self.relative_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct BoundOutputScope {
    relative_path: String,
    device: u64,
    inode: u64,
    unix_mode: u32,
    owner_user_id: u32,
    owner_group_id: u32,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
    ancestors: Vec<DirectoryIdentity>,
}

impl BoundOutputScope {
    pub(crate) fn relative_path(&self) -> &str {
        &self.relative_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct BoundCatalogInvocation {
    invocation_id: String,
    catalog_id: String,
    graph_id: String,
    candidate_id: String,
    plan_id: String,
    definition_id: String,
    definition_sha256: String,
    node_id: String,
    depends_on: Vec<String>,
    selected_tool: String,
    selected_tool_identity_sha256: String,
    used_fallback: bool,
    input_id: String,
    program_path_hex: String,
    program_sha256: String,
    program_byte_length: u64,
    program_unix_mode: u32,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    read_sources: Vec<BoundReadSource>,
    timeout_ms: u64,
    output_budget_bytes: u64,
    output_scopes: Vec<BoundOutputScope>,
    runner_policy: &'static str,
    read_policy: &'static str,
}

impl BoundCatalogInvocation {
    pub(crate) fn invocation_id(&self) -> &str {
        &self.invocation_id
    }

    pub(crate) fn node_id(&self) -> &str {
        &self.node_id
    }

    pub(crate) fn selected_tool(&self) -> &str {
        &self.selected_tool
    }

    pub(crate) fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub(crate) fn environment(&self) -> &BTreeMap<String, String> {
        &self.environment
    }

    pub(crate) fn read_sources(&self) -> &[BoundReadSource] {
        &self.read_sources
    }

    pub(crate) fn timeout_ms(&self) -> u64 {
        self.timeout_ms
    }

    pub(crate) fn output_budget_bytes(&self) -> u64 {
        self.output_budget_bytes
    }

    pub(crate) fn output_scopes(&self) -> &[BoundOutputScope] {
        &self.output_scopes
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct BoundRoutineInvocationSet {
    invocation_set_id: String,
    catalog_id: String,
    graph_id: String,
    candidate_id: String,
    plan_id: String,
    invocations: Vec<BoundCatalogInvocation>,
}

impl BoundRoutineInvocationSet {
    pub(crate) fn invocation_set_id(&self) -> &str {
        &self.invocation_set_id
    }

    pub(crate) fn invocations(&self) -> &[BoundCatalogInvocation] {
        &self.invocations
    }
}

#[derive(Debug)]
pub(crate) struct ProductionRoutineCatalog {
    source: SealedFile,
    catalog_id: String,
    graph_id: String,
    candidate_id: String,
    definitions: BTreeMap<String, RoutineDefinition>,
}

impl ProductionRoutineCatalog {
    pub(crate) fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    pub(crate) fn graph_id(&self) -> &str {
        &self.graph_id
    }

    pub(crate) fn definition_count(&self) -> usize {
        self.definitions.len()
    }

    pub(crate) fn definition_ids(&self) -> impl Iterator<Item = &str> {
        self.definitions
            .values()
            .map(|row| row.definition_id.as_str())
    }

    pub(crate) fn verify_current(&self) -> CatalogResult<()> {
        self.source.verify_current(MAX_CATALOG_BYTES)
    }

    pub(crate) fn bind_selected(
        &self,
        request: CatalogSelectionRequest,
    ) -> CatalogResult<BoundRoutineInvocationSet> {
        self.verify_current()?;
        if request.catalog_id != self.catalog_id
            || request.graph_id != self.graph_id
            || request.candidate_id != self.candidate_id
        {
            return Err(error("catalog-selection-binding-stale"));
        }
        let selected_ids = request
            .selected
            .iter()
            .map(|row| row.node_id.clone())
            .collect::<BTreeSet<_>>();
        let canonical_order = canonical_selected_order(&self.definitions, &selected_ids)?;
        if request
            .selected
            .iter()
            .map(|row| row.node_id.as_str())
            .ne(canonical_order.iter().map(String::as_str))
        {
            return Err(error("catalog-selection-order-stale"));
        }

        let mut invocations = Vec::with_capacity(request.selected.len());
        let mut read_seals = Vec::new();
        let mut output_seals = Vec::new();
        let mut runner_seals = BTreeMap::<String, SealedFile>::new();

        for selected in &request.selected {
            let definition = self
                .definitions
                .get(&selected.node_id)
                .ok_or_else(|| error("catalog-selection-node-unknown"))?;
            if selected.depends_on != definition.depends_on {
                return Err(error("catalog-selection-dependencies-stale"));
            }
            let recipe = definition.recipe(selected.used_fallback)?;
            if selected.selected_tool != recipe.tool {
                return Err(error("catalog-selection-runner-mismatch"));
            }
            if selected.selected_tool_identity_sha256 != recipe.tool_identity_sha256 {
                return Err(error("catalog-selection-runner-authority-stale"));
            }
            let runner = request
                .runners
                .get(&selected.selected_tool)
                .ok_or_else(|| error("catalog-selection-runner-unobserved"))?;
            if runner.tool_identity_sha256 != recipe.tool_identity_sha256
                || runner.executable_path != recipe.executable_path
                || runner.program_sha256 != recipe.program_sha256
                || runner.program_byte_length != recipe.program_byte_length
                || runner.program_unix_mode != recipe.program_unix_mode
            {
                return Err(error("catalog-runner-authority-stale"));
            }
            let runner_seal = capture_program(runner)?;
            runner_seals.insert(selected.selected_tool.clone(), runner_seal.clone());

            if selected.transitive_inputs.len() != definition.read_sources.len()
                || selected
                    .transitive_inputs
                    .iter()
                    .map(|input| input.relative_path.as_str())
                    .ne(definition.read_sources.iter().map(CatalogPath::as_str))
            {
                return Err(error("catalog-selection-transitive-input-set-inexact"));
            }

            let mut total_read_bytes = 0_u64;
            let mut bound_reads = Vec::with_capacity(selected.transitive_inputs.len());
            for expected in &selected.transitive_inputs {
                let seal = SealedFile::capture_relative(
                    &self.source.root,
                    &expected.relative_path,
                    MAX_READ_SOURCE_BYTES,
                )?;
                total_read_bytes = total_read_bytes
                    .checked_add(seal.identity.byte_length)
                    .ok_or_else(|| error("catalog-selection-read-budget-exceeded"))?;
                if total_read_bytes > MAX_READ_SOURCE_BYTES
                    || seal.identity.sha256 != expected.sha256
                    || seal.identity.byte_length != expected.byte_length
                {
                    return Err(error("catalog-selection-transitive-input-stale"));
                }
                bound_reads.push(seal.bound_read_source(expected.relative_path.as_str()));
                read_seals.push(seal);
            }

            let mut bound_outputs = Vec::with_capacity(definition.output_scopes.len());
            for relative in &definition.output_scopes {
                let seal = SealedDirectory::capture(&self.source.root, relative)?;
                bound_outputs.push(seal.bound_output_scope(relative.as_str()));
                output_seals.push(seal);
            }

            let program_parent = recipe
                .executable_path
                .parent()
                .and_then(Path::to_str)
                .ok_or_else(|| error("catalog-runner-parent-invalid"))?;
            let mut environment = BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("LC_ALL".to_owned(), "C".to_owned()),
                ("PATH".to_owned(), program_parent.to_owned()),
            ]);
            for (key, value) in &definition.environment {
                if environment.insert(key.clone(), value.clone()).is_some() {
                    return Err(error("catalog-environment-default-override"));
                }
            }
            validate_bound_environment(&environment)?;

            let mut invocation = BoundCatalogInvocation {
                invocation_id: String::new(),
                catalog_id: self.catalog_id.clone(),
                graph_id: self.graph_id.clone(),
                candidate_id: self.candidate_id.clone(),
                plan_id: request.plan_id.clone(),
                definition_id: definition.definition_id.clone(),
                definition_sha256: definition.definition_sha256.clone(),
                node_id: definition.node_id.clone(),
                depends_on: definition.depends_on.iter().cloned().collect(),
                selected_tool: recipe.tool.clone(),
                selected_tool_identity_sha256: recipe.tool_identity_sha256.clone(),
                used_fallback: selected.used_fallback,
                input_id: selected.input_id.clone(),
                program_path_hex: hex_path(&recipe.executable_path)?,
                program_sha256: recipe.program_sha256.clone(),
                program_byte_length: recipe.program_byte_length,
                program_unix_mode: recipe.program_unix_mode,
                arguments: recipe.arguments.clone(),
                environment,
                read_sources: bound_reads,
                timeout_ms: definition.timeout_ms,
                output_budget_bytes: definition.output_budget_bytes,
                output_scopes: bound_outputs,
                runner_policy: RUNNER_POLICY,
                read_policy: READ_POLICY,
            };
            invocation.invocation_id = digest_json(&InvocationIdentity::from(&invocation))?;
            invocations.push(invocation);
        }

        self.verify_current()?;
        for seal in &read_seals {
            seal.verify_current(MAX_READ_SOURCE_BYTES)?;
        }
        for seal in runner_seals.values() {
            seal.verify_current(MAX_READ_SOURCE_BYTES)?;
        }
        for seal in &output_seals {
            seal.verify_current()?;
        }

        let invocation_set_id = digest_json(&InvocationSetIdentity {
            catalog_id: &self.catalog_id,
            graph_id: &self.graph_id,
            candidate_id: &self.candidate_id,
            plan_id: &request.plan_id,
            invocation_ids: invocations
                .iter()
                .map(|row| row.invocation_id.as_str())
                .collect(),
        })?;
        Ok(BoundRoutineInvocationSet {
            invocation_set_id,
            catalog_id: self.catalog_id.clone(),
            graph_id: self.graph_id.clone(),
            candidate_id: self.candidate_id.clone(),
            plan_id: request.plan_id,
            invocations,
        })
    }
}

fn canonical_selected_order(
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
struct RawCatalog {
    schema_version: String,
    graph_id: String,
    routines: Vec<RawRoutineDefinition>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRoutineDefinition {
    definition_id: String,
    node_id: String,
    depends_on: Vec<String>,
    working_directory: String,
    runner_policy: String,
    read_policy: String,
    read_sources: Vec<String>,
    environment: BTreeMap<String, String>,
    timeout_ms: u64,
    output_budget_bytes: u64,
    output_scopes: Vec<String>,
    primary: RawRunnerRecipe,
    fallback: Option<RawFallbackRecipe>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRunnerRecipe {
    tool: String,
    tool_identity_sha256: String,
    executable_path: String,
    program_sha256: String,
    program_byte_length: u64,
    program_unix_mode: u32,
    arguments: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFallbackRecipe {
    tool: String,
    tool_identity_sha256: String,
    executable_path: String,
    program_sha256: String,
    program_byte_length: u64,
    program_unix_mode: u32,
    arguments: Vec<String>,
    equivalence: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct RunnerRecipe {
    tool: String,
    tool_identity_sha256: String,
    executable_path: PathBuf,
    program_sha256: String,
    program_byte_length: u64,
    program_unix_mode: u32,
    arguments: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct RoutineDefinition {
    definition_id: String,
    definition_sha256: String,
    node_id: String,
    depends_on: BTreeSet<String>,
    read_sources: Vec<CatalogPath>,
    environment: BTreeMap<String, String>,
    timeout_ms: u64,
    output_budget_bytes: u64,
    output_scopes: Vec<CatalogPath>,
    primary: RunnerRecipe,
    fallback: Option<RunnerRecipe>,
}

impl RoutineDefinition {
    fn recipe(&self, fallback: bool) -> CatalogResult<&RunnerRecipe> {
        if fallback {
            self.fallback
                .as_ref()
                .ok_or_else(|| error("catalog-selection-fallback-unavailable"))
        } else {
            Ok(&self.primary)
        }
    }
}

fn validate_raw_catalog(
    raw: RawCatalog,
    adoption: &CatalogAdoption,
) -> CatalogResult<BTreeMap<String, RoutineDefinition>> {
    if raw.schema_version != CATALOG_SCHEMA {
        return Err(error("catalog-schema-version-unsupported"));
    }
    if raw.graph_id != adoption.graph_id {
        return Err(error("catalog-graph-binding-stale"));
    }
    if raw.routines.len() != adoption.nodes.len()
        || raw.routines.is_empty()
        || raw.routines.len() > MAX_DEFINITIONS
    {
        return Err(error("catalog-definition-set-inexact"));
    }

    let mut definitions = BTreeMap::new();
    let mut definition_ids = BTreeSet::new();
    let mut definition_case_ids = BTreeSet::new();
    let mut node_case_ids = BTreeSet::new();
    for raw_definition in raw.routines {
        let definition_id = identifier(raw_definition.definition_id)?;
        let node_id = identifier(raw_definition.node_id)?;
        if !definition_ids.insert(definition_id.clone()) {
            return Err(error("catalog-definition-id-duplicated"));
        }
        if !definition_case_ids.insert(definition_id.to_ascii_lowercase()) {
            return Err(error("catalog-definition-id-ambiguous"));
        }
        if !node_case_ids.insert(node_id.to_ascii_lowercase()) {
            return Err(error("catalog-definition-node-ambiguous"));
        }
        if definitions.contains_key(&node_id) {
            return Err(error("catalog-definition-node-duplicated"));
        }
        let adopted = adoption
            .nodes
            .get(&node_id)
            .ok_or_else(|| error("catalog-definition-node-unknown"))?;
        let depends_on = identifier_set(
            raw_definition.depends_on,
            "catalog-definition-dependency-duplicated",
        )?;
        if depends_on != adopted.depends_on {
            return Err(error("catalog-definition-dependencies-mismatch"));
        }
        if raw_definition.working_directory != "." {
            return Err(error("catalog-working-directory-unsafe"));
        }
        if raw_definition.runner_policy != RUNNER_POLICY {
            return Err(error("catalog-runner-policy-unsupported"));
        }
        if raw_definition.read_policy != READ_POLICY {
            return Err(error("catalog-read-policy-unsupported"));
        }
        let primary = validate_recipe(raw_definition.primary, &adopted.primary_tool)?;
        let fallback = match (raw_definition.fallback, &adopted.fallback_tool) {
            (None, None) => None,
            (Some(raw_fallback), Some(expected)) => {
                if raw_fallback.equivalence != FALLBACK_EQUIVALENCE {
                    return Err(error("catalog-fallback-equivalence-unproven"));
                }
                Some(validate_recipe(
                    RawRunnerRecipe {
                        tool: raw_fallback.tool,
                        tool_identity_sha256: raw_fallback.tool_identity_sha256,
                        executable_path: raw_fallback.executable_path,
                        program_sha256: raw_fallback.program_sha256,
                        program_byte_length: raw_fallback.program_byte_length,
                        program_unix_mode: raw_fallback.program_unix_mode,
                        arguments: raw_fallback.arguments,
                    },
                    expected,
                )?)
            }
            _ => return Err(error("catalog-fallback-definition-mismatch")),
        };
        if fallback
            .as_ref()
            .is_some_and(|row| row.tool.eq_ignore_ascii_case(&primary.tool))
        {
            return Err(error("catalog-fallback-runner-duplicated"));
        }

        let read_sources = normalize_catalog_paths(
            raw_definition.read_sources,
            MAX_READ_SOURCES,
            false,
            "catalog-read-source",
        )?;
        if read_sources.is_empty() {
            return Err(error("catalog-read-source-set-empty"));
        }
        let output_scopes = normalize_catalog_paths(
            raw_definition.output_scopes,
            MAX_OUTPUT_SCOPES,
            true,
            "catalog-output-scope",
        )?;
        if output_scopes.iter().any(|output| {
            read_sources
                .iter()
                .any(|input| paths_overlap(output, input))
        }) {
            return Err(error("catalog-output-scope-overlaps-input"));
        }
        validate_environment(&raw_definition.environment)?;
        if raw_definition.timeout_ms == 0 || raw_definition.timeout_ms > MAX_TIMEOUT_MS {
            return Err(error("catalog-timeout-invalid"));
        }
        if raw_definition.output_budget_bytes == 0
            || raw_definition.output_budget_bytes > MAX_OUTPUT_BUDGET_BYTES
        {
            return Err(error("catalog-output-budget-invalid"));
        }
        let mut definition = RoutineDefinition {
            definition_id,
            definition_sha256: String::new(),
            node_id: node_id.clone(),
            depends_on,
            read_sources,
            environment: raw_definition.environment,
            timeout_ms: raw_definition.timeout_ms,
            output_budget_bytes: raw_definition.output_budget_bytes,
            output_scopes,
            primary,
            fallback,
        };
        definition.definition_sha256 = digest_json(&DefinitionIdentity::from(&definition))?;
        definitions.insert(node_id, definition);
    }
    if definitions.keys().ne(adoption.nodes.keys()) {
        return Err(error("catalog-definition-set-inexact"));
    }
    validate_global_path_compatibility(&definitions)?;
    validate_global_runner_compatibility(&definitions)?;
    Ok(definitions)
}

fn validate_global_path_compatibility(
    definitions: &BTreeMap<String, RoutineDefinition>,
) -> CatalogResult<()> {
    let read_sources = definitions
        .values()
        .flat_map(|definition| definition.read_sources.iter())
        .collect::<Vec<_>>();
    let mut read_case_paths = BTreeMap::new();
    for source in &read_sources {
        let case_key = source.as_str().to_ascii_lowercase();
        if read_case_paths
            .insert(case_key, source.as_str())
            .is_some_and(|prior| prior != source.as_str())
        {
            return Err(error("catalog-read-source-global-ambiguous"));
        }
    }
    let mut output_scopes = definitions
        .values()
        .flat_map(|definition| definition.output_scopes.iter())
        .collect::<Vec<_>>();
    output_scopes.sort();
    output_scopes.dedup();
    for (index, left) in output_scopes.iter().enumerate() {
        if output_scopes[index + 1..]
            .iter()
            .any(|right| paths_overlap(left, right))
        {
            return Err(error("catalog-output-scope-global-overlap"));
        }
        if read_sources.iter().any(|input| paths_overlap(left, input)) {
            return Err(error("catalog-output-scope-global-input-overlap"));
        }
    }
    Ok(())
}

fn validate_recipe(raw: RawRunnerRecipe, expected_tool: &str) -> CatalogResult<RunnerRecipe> {
    let tool = identifier(raw.tool)?;
    if tool != expected_tool {
        return Err(error("catalog-runner-registry-mismatch"));
    }
    let executable_path = absolute_program_path(raw.executable_path)?;
    let tool_identity_sha256 = required_sha256(
        raw.tool_identity_sha256,
        "catalog-runner-tool-identity-invalid",
    )?;
    let program_sha256 =
        required_sha256(raw.program_sha256, "catalog-runner-program-digest-invalid")?;
    if raw.program_byte_length == 0 || raw.program_byte_length > MAX_READ_SOURCE_BYTES {
        return Err(error("catalog-runner-program-length-invalid"));
    }
    if raw.program_unix_mode & 0o170000 != 0o100000
        || raw.program_unix_mode & 0o111 == 0
        || raw.program_unix_mode & 0o022 != 0
    {
        return Err(error("catalog-runner-program-mode-invalid"));
    }
    validate_arguments(&raw.arguments)?;
    Ok(RunnerRecipe {
        tool,
        tool_identity_sha256,
        executable_path,
        program_sha256,
        program_byte_length: raw.program_byte_length,
        program_unix_mode: raw.program_unix_mode,
        arguments: raw.arguments,
    })
}

fn absolute_program_path(value: String) -> CatalogResult<PathBuf> {
    let path = PathBuf::from(&value);
    if value.len() > 4096
        || value.ends_with('/')
        || value.contains("//")
        || value.bytes().any(|byte| byte.is_ascii_control())
        || !path.is_absolute()
        || path.components().any(|component| {
            !matches!(
                component,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
    {
        return Err(error("catalog-runner-path-invalid"));
    }
    Ok(path)
}

fn validate_global_runner_compatibility(
    definitions: &BTreeMap<String, RoutineDefinition>,
) -> CatalogResult<()> {
    let mut authorities = BTreeMap::<String, (&str, (&str, &Path, &str, u64, u32))>::new();
    for recipe in definitions.values().flat_map(|definition| {
        std::iter::once(&definition.primary).chain(definition.fallback.as_ref())
    }) {
        let authority = (
            recipe.tool_identity_sha256.as_str(),
            recipe.executable_path.as_path(),
            recipe.program_sha256.as_str(),
            recipe.program_byte_length,
            recipe.program_unix_mode,
        );
        let canonical_tool = recipe.tool.to_ascii_lowercase();
        if let Some((accepted_spelling, accepted_authority)) = authorities.get(&canonical_tool) {
            if *accepted_spelling != recipe.tool {
                return Err(error("catalog-runner-spelling-ambiguous"));
            }
            if *accepted_authority != authority {
                return Err(error("catalog-runner-authority-ambiguous"));
            }
        } else {
            authorities.insert(canonical_tool, (recipe.tool.as_str(), authority));
        }
    }
    Ok(())
}

fn validate_arguments(arguments: &[String]) -> CatalogResult<()> {
    let total = arguments.iter().map(String::len).sum::<usize>();
    if arguments.len() > MAX_ARGUMENTS
        || total > MAX_ARGUMENT_BYTES
        || arguments.iter().any(|argument| {
            argument.is_empty()
                || argument.len() > 4_096
                || argument.bytes().any(|byte| byte.is_ascii_control())
        })
    {
        return Err(error("catalog-arguments-invalid"));
    }
    Ok(())
}

fn validate_environment(environment: &BTreeMap<String, String>) -> CatalogResult<()> {
    let total = environment
        .iter()
        .map(|(key, value)| key.len().saturating_add(value.len()))
        .sum::<usize>();
    if environment.len() > MAX_ENVIRONMENT_ENTRIES
        || total > MAX_ENVIRONMENT_BYTES
        || environment.iter().any(|(key, value)| {
            key.is_empty()
                || key.len() > 128
                || matches!(key.as_str(), "LANG" | "LC_ALL" | "PATH")
                || key.starts_with("HUL_ROUTINE_")
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
                || value.len() > 4_096
                || value
                    .bytes()
                    .any(|byte| byte == 0 || byte.is_ascii_control())
                || startup_loader_environment_key(key)
        })
    {
        return Err(error("catalog-environment-invalid"));
    }
    Ok(())
}

fn validate_bound_environment(environment: &BTreeMap<String, String>) -> CatalogResult<()> {
    let total = environment
        .iter()
        .map(|(key, value)| key.len().saturating_add(value.len()))
        .sum::<usize>();
    if environment.len() > 64
        || total > 64 * 1024
        || environment.iter().any(|(key, value)| {
            key.is_empty()
                || key.len() > 128
                || key.starts_with("HUL_ROUTINE_")
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
                || value.len() > 4_096
                || value
                    .bytes()
                    .any(|byte| byte == 0 || byte.is_ascii_control())
                || startup_loader_environment_key(key)
        })
    {
        return Err(error("catalog-bound-environment-invalid"));
    }
    Ok(())
}

fn startup_loader_environment_key(key: &str) -> bool {
    const EXACT: &[&str] = &[
        "BASH_ENV",
        "BASH_LOADABLES_PATH",
        "CLASSPATH",
        "ENV",
        "GEM_HOME",
        "GEM_PATH",
        "JDK_JAVA_OPTIONS",
        "LD_AUDIT",
        "LD_LIBRARY_PATH",
        "LD_PRELOAD",
        "LIBPATH",
        "NODE_OPTIONS",
        "NODE_PATH",
        "PERL5LIB",
        "PERL5OPT",
        "PERLLIB",
        "PHP_INI_SCAN_DIR",
        "PHPRC",
        "PYTHONBREAKPOINT",
        "PYTHONHOME",
        "PYTHONINSPECT",
        "PYTHONPATH",
        "PYTHONSTARTUP",
        "PYTHONUSERBASE",
        "RUBYLIB",
        "RUBYOPT",
        "RUBYPATH",
        "SHLIB_PATH",
        "ZDOTDIR",
    ];
    EXACT.contains(&key)
        || key.starts_with("DYLD_")
        || key.starts_with("LD_PRELOAD_")
        || key.ends_with("_STARTUP")
        || key.ends_with("_TOOL_OPTIONS")
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
struct CatalogPath(String);

impl CatalogPath {
    fn parse(value: String) -> CatalogResult<Self> {
        let depth = value.split('/').count();
        if value.is_empty()
            || value.len() > 4_096
            || depth > 64
            || value.starts_with('/')
            || value.ends_with('/')
            || value.contains("//")
            || value.contains('\\')
            || value.bytes().any(|byte| byte.is_ascii_control())
            || value
                .split('/')
                .any(|component| component.is_empty() || matches!(component, "." | ".."))
        {
            return Err(error("catalog-repository-relative-path-required"));
        }
        Ok(Self(value))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

fn normalize_catalog_paths(
    values: Vec<String>,
    maximum: usize,
    reject_overlaps: bool,
    label: &'static str,
) -> CatalogResult<Vec<CatalogPath>> {
    if values.len() > maximum {
        return Err(error(match label {
            "catalog-read-source" => "catalog-read-source-limit-exceeded",
            _ => "catalog-output-scope-limit-exceeded",
        }));
    }
    let mut paths = values
        .into_iter()
        .map(CatalogPath::parse)
        .collect::<CatalogResult<Vec<_>>>()?;
    paths.sort();
    let mut case_paths = BTreeSet::new();
    if paths
        .iter()
        .any(|path| !case_paths.insert(path.as_str().to_ascii_lowercase()))
    {
        return Err(error(match label {
            "catalog-read-source" => "catalog-read-source-ambiguous",
            _ => "catalog-output-scope-ambiguous",
        }));
    }
    if reject_overlaps {
        for (index, left) in paths.iter().enumerate() {
            if paths[index + 1..]
                .iter()
                .any(|right| paths_overlap(left, right))
            {
                return Err(error("catalog-output-scope-overlap"));
            }
        }
    }
    Ok(paths)
}

fn normalize_input_expectations(inputs: &mut Vec<TransitiveInputExpectation>) -> CatalogResult<()> {
    if inputs.is_empty() || inputs.len() > MAX_READ_SOURCES {
        return Err(error("catalog-selection-input-count-invalid"));
    }
    inputs.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let mut case_paths = BTreeSet::new();
    if inputs
        .iter()
        .any(|input| !case_paths.insert(input.relative_path.as_str().to_ascii_lowercase()))
    {
        return Err(error("catalog-selection-input-ambiguous"));
    }
    Ok(())
}

fn paths_overlap(left: &CatalogPath, right: &CatalogPath) -> bool {
    let left = left.as_str().to_ascii_lowercase();
    let right = right.as_str().to_ascii_lowercase();
    left == right
        || right
            .strip_prefix(&left)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || left
            .strip_prefix(&right)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct DirectoryIdentity {
    relative_directory: String,
    device: u64,
    inode: u64,
    unix_mode: u32,
    owner_user_id: u32,
    owner_group_id: u32,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    unix_mode: u32,
    owner_user_id: u32,
    owner_group_id: u32,
    link_count: u64,
    byte_length: u64,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
    sha256: String,
    ancestors: Vec<DirectoryIdentity>,
}

#[derive(Clone, Debug)]
struct SealedFile {
    root: PathBuf,
    relative: Option<CatalogPath>,
    absolute: PathBuf,
    root_identity: DirectoryIdentity,
    identity: FileIdentity,
    bytes: Vec<u8>,
}

impl SealedFile {
    fn capture_relative(root: &Path, relative: &CatalogPath, maximum: u64) -> CatalogResult<Self> {
        let root_identity = capture_root(root)?;
        let absolute = root.join(relative.as_str());
        let (identity, bytes) = capture_regular_file(
            root,
            &root_identity,
            &absolute,
            Some(relative),
            maximum,
            true,
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            relative: Some(relative.clone()),
            absolute,
            root_identity,
            identity,
            bytes,
        })
    }

    fn verify_current(&self, maximum: u64) -> CatalogResult<()> {
        let current_root = capture_root(&self.root)?;
        if current_root != self.root_identity {
            return Err(error("catalog-root-identity-changed"));
        }
        let (identity, bytes) = capture_regular_file(
            &self.root,
            &self.root_identity,
            &self.absolute,
            self.relative.as_ref(),
            maximum,
            self.relative.is_some(),
        )?;
        if identity != self.identity || bytes != self.bytes {
            return Err(error("catalog-sealed-file-changed"));
        }
        Ok(())
    }

    fn bound_read_source(&self, relative_path: &str) -> BoundReadSource {
        BoundReadSource {
            relative_path: relative_path.to_owned(),
            device: self.identity.device,
            inode: self.identity.inode,
            unix_mode: self.identity.unix_mode,
            owner_user_id: self.identity.owner_user_id,
            owner_group_id: self.identity.owner_group_id,
            link_count: self.identity.link_count,
            byte_length: self.identity.byte_length,
            modified_seconds: self.identity.modified_seconds,
            modified_nanos: self.identity.modified_nanos,
            changed_seconds: self.identity.changed_seconds,
            changed_nanos: self.identity.changed_nanos,
            sha256: self.identity.sha256.clone(),
            ancestors: self.identity.ancestors.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct SealedDirectory {
    root: PathBuf,
    relative: CatalogPath,
    root_identity: DirectoryIdentity,
    identity: DirectoryIdentity,
    ancestors: Vec<DirectoryIdentity>,
}

impl SealedDirectory {
    fn capture(root: &Path, relative: &CatalogPath) -> CatalogResult<Self> {
        let root_identity = capture_root(root)?;
        let (identity, ancestors) = capture_directory(root, &root_identity, relative)?;
        let current_root = capture_root(root)?;
        let (current_identity, current_ancestors) =
            capture_directory(root, &current_root, relative)?;
        if current_root != root_identity
            || current_identity != identity
            || current_ancestors != ancestors
        {
            return Err(error("catalog-output-scope-capture-race"));
        }
        Ok(Self {
            root: root.to_path_buf(),
            relative: relative.clone(),
            root_identity,
            identity,
            ancestors,
        })
    }

    fn verify_current(&self) -> CatalogResult<()> {
        let current_root = capture_root(&self.root)?;
        if current_root != self.root_identity {
            return Err(error("catalog-root-identity-changed"));
        }
        let (identity, ancestors) = capture_directory(&self.root, &current_root, &self.relative)?;
        if identity != self.identity || ancestors != self.ancestors {
            return Err(error("catalog-output-scope-changed"));
        }
        Ok(())
    }

    fn bound_output_scope(&self, relative_path: &str) -> BoundOutputScope {
        BoundOutputScope {
            relative_path: relative_path.to_owned(),
            device: self.identity.device,
            inode: self.identity.inode,
            unix_mode: self.identity.unix_mode,
            owner_user_id: self.identity.owner_user_id,
            owner_group_id: self.identity.owner_group_id,
            modified_seconds: self.identity.modified_seconds,
            modified_nanos: self.identity.modified_nanos,
            changed_seconds: self.identity.changed_seconds,
            changed_nanos: self.identity.changed_nanos,
            ancestors: self.ancestors.clone(),
        }
    }
}

#[cfg(unix)]
fn capture_root(root: &Path) -> CatalogResult<DirectoryIdentity> {
    if !root.is_absolute() || root.to_str().is_none() {
        return Err(error("catalog-worktree-root-invalid"));
    }
    let metadata =
        fs::symlink_metadata(root).map_err(|_| error("catalog-worktree-root-missing"))?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(error("catalog-worktree-root-unsafe"));
    }
    Ok(directory_identity(".", &metadata))
}

#[cfg(not(unix))]
fn capture_root(_root: &Path) -> CatalogResult<DirectoryIdentity> {
    Err(error("catalog-platform-unsupported"))
}

#[cfg(unix)]
fn capture_regular_file(
    root: &Path,
    root_identity: &DirectoryIdentity,
    absolute: &Path,
    relative: Option<&CatalogPath>,
    maximum: u64,
    require_root_device: bool,
) -> CatalogResult<(FileIdentity, Vec<u8>)> {
    let ancestors = if let Some(relative) = relative {
        capture_ancestors(root, root_identity, relative)?
    } else {
        capture_absolute_ancestors(absolute)?
    };
    let before = fs::symlink_metadata(absolute).map_err(|_| error("catalog-file-missing"))?;
    validate_regular_metadata(&before, root_identity.device, maximum, require_root_device)?;
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    let mut file = options
        .open(absolute)
        .map_err(|_| error("catalog-file-open-failed"))?;
    let opened = file
        .metadata()
        .map_err(|_| error("catalog-file-metadata-failed"))?;
    validate_regular_metadata(&opened, root_identity.device, maximum, require_root_device)?;
    if file_metadata_tuple(&before) != file_metadata_tuple(&opened) {
        return Err(error("catalog-file-open-race"));
    }
    let mut bytes = Vec::with_capacity((opened.len().min(maximum)) as usize);
    file.by_ref()
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| error("catalog-file-read-failed"))?;
    if bytes.len() as u64 > maximum || bytes.len() as u64 != opened.len() {
        return Err(error("catalog-file-size-invalid"));
    }
    let after = file
        .metadata()
        .map_err(|_| error("catalog-file-metadata-failed"))?;
    if file_metadata_tuple(&opened) != file_metadata_tuple(&after) {
        return Err(error("catalog-file-read-race"));
    }
    let current_root = capture_root(root)?;
    let current_ancestors = if let Some(relative) = relative {
        capture_ancestors(root, &current_root, relative)?
    } else {
        capture_absolute_ancestors(absolute)?
    };
    if &current_root != root_identity || current_ancestors != ancestors {
        return Err(error("catalog-file-ancestor-race"));
    }
    Ok((
        FileIdentity {
            device: opened.dev(),
            inode: opened.ino(),
            unix_mode: opened.mode(),
            owner_user_id: opened.uid(),
            owner_group_id: opened.gid(),
            link_count: opened.nlink(),
            byte_length: opened.len(),
            modified_seconds: opened.mtime(),
            modified_nanos: opened.mtime_nsec(),
            changed_seconds: opened.ctime(),
            changed_nanos: opened.ctime_nsec(),
            sha256: sha256(&bytes),
            ancestors,
        },
        bytes,
    ))
}

#[cfg(not(unix))]
fn capture_regular_file(
    _root: &Path,
    _root_identity: &DirectoryIdentity,
    _absolute: &Path,
    _relative: Option<&CatalogPath>,
    _maximum: u64,
    _require_root_device: bool,
) -> CatalogResult<(FileIdentity, Vec<u8>)> {
    Err(error("catalog-platform-unsupported"))
}

#[cfg(unix)]
fn validate_regular_metadata(
    metadata: &fs::Metadata,
    expected_device: u64,
    maximum: u64,
    require_expected_device: bool,
) -> CatalogResult<()> {
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || (require_expected_device && metadata.dev() != expected_device)
    {
        return Err(error("catalog-file-object-unsafe"));
    }
    if metadata.len() > maximum {
        return Err(error("catalog-file-size-invalid"));
    }
    Ok(())
}

#[cfg(unix)]
fn capture_program(observation: &RunnerObservation) -> CatalogResult<SealedFile> {
    let root_path = Path::new("/");
    let root_identity = capture_root(root_path)?;
    let (identity, bytes) = capture_regular_file(
        root_path,
        &root_identity,
        &observation.executable_path,
        None,
        MAX_READ_SOURCE_BYTES,
        false,
    )?;
    if identity.sha256 != observation.program_sha256
        || identity.byte_length != observation.program_byte_length
        || identity.unix_mode != observation.program_unix_mode
    {
        return Err(error("catalog-runner-program-identity-stale"));
    }
    let effective_user = unsafe { libc::geteuid() };
    if identity.owner_user_id == effective_user
        || identity.unix_mode & 0o111 == 0
        || identity.unix_mode & 0o022 != 0
    {
        return Err(error("catalog-runner-program-mutable"));
    }
    Ok(SealedFile {
        root: root_path.to_path_buf(),
        relative: None,
        absolute: observation.executable_path.clone(),
        root_identity,
        identity,
        bytes,
    })
}

#[cfg(not(unix))]
fn capture_program(_observation: &RunnerObservation) -> CatalogResult<SealedFile> {
    Err(error("catalog-platform-unsupported"))
}

#[cfg(unix)]
fn capture_ancestors(
    root: &Path,
    root_identity: &DirectoryIdentity,
    relative: &CatalogPath,
) -> CatalogResult<Vec<DirectoryIdentity>> {
    let mut result = vec![root_identity.clone()];
    let mut current = root.to_path_buf();
    let components = relative.as_str().split('/').collect::<Vec<_>>();
    for (index, component) in components[..components.len().saturating_sub(1)]
        .iter()
        .enumerate()
    {
        current.push(component);
        let metadata =
            fs::symlink_metadata(&current).map_err(|_| error("catalog-file-ancestor-missing"))?;
        if !metadata.file_type().is_dir()
            || metadata.file_type().is_symlink()
            || metadata.dev() != root_identity.device
        {
            return Err(error("catalog-file-ancestor-unsafe"));
        }
        result.push(directory_identity(
            &components[..=index].join("/"),
            &metadata,
        ));
    }
    Ok(result)
}

#[cfg(unix)]
fn capture_absolute_ancestors(path: &Path) -> CatalogResult<Vec<DirectoryIdentity>> {
    let parent = path
        .parent()
        .ok_or_else(|| error("catalog-runner-parent-invalid"))?;
    let mut result = Vec::new();
    let mut current = PathBuf::from("/");
    let root_metadata =
        fs::symlink_metadata(&current).map_err(|_| error("catalog-runner-ancestor-missing"))?;
    result.push(directory_identity("/", &root_metadata));
    for component in parent.components().skip(1) {
        current.push(component.as_os_str());
        let metadata =
            fs::symlink_metadata(&current).map_err(|_| error("catalog-runner-ancestor-missing"))?;
        if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
            return Err(error("catalog-runner-ancestor-unsafe"));
        }
        let text = current
            .to_str()
            .ok_or_else(|| error("catalog-runner-path-invalid"))?;
        result.push(directory_identity(text, &metadata));
    }
    Ok(result)
}

#[cfg(unix)]
fn capture_directory(
    root: &Path,
    root_identity: &DirectoryIdentity,
    relative: &CatalogPath,
) -> CatalogResult<(DirectoryIdentity, Vec<DirectoryIdentity>)> {
    let mut current = root.to_path_buf();
    let mut ancestors = vec![root_identity.clone()];
    let components = relative.as_str().split('/').collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        current.push(component);
        let metadata =
            fs::symlink_metadata(&current).map_err(|_| error("catalog-output-scope-missing"))?;
        if !metadata.file_type().is_dir()
            || metadata.file_type().is_symlink()
            || metadata.dev() != root_identity.device
        {
            return Err(error("catalog-output-scope-unsafe"));
        }
        let identity = directory_identity(&components[..=index].join("/"), &metadata);
        if index + 1 == components.len() {
            return Ok((identity, ancestors));
        }
        ancestors.push(identity);
    }
    Err(error("catalog-output-scope-missing"))
}

#[cfg(unix)]
fn directory_identity(relative: &str, metadata: &fs::Metadata) -> DirectoryIdentity {
    DirectoryIdentity {
        relative_directory: relative.to_owned(),
        device: metadata.dev(),
        inode: metadata.ino(),
        unix_mode: metadata.mode(),
        owner_user_id: metadata.uid(),
        owner_group_id: metadata.gid(),
        modified_seconds: metadata.mtime(),
        modified_nanos: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanos: metadata.ctime_nsec(),
    }
}

#[cfg(unix)]
fn file_metadata_tuple(
    metadata: &fs::Metadata,
) -> (u64, u64, u32, u32, u32, u64, u64, i64, i64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.mode(),
        metadata.uid(),
        metadata.gid(),
        metadata.nlink(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

#[derive(Serialize)]
struct CatalogIdentity<'a> {
    source_sha256: &'a str,
    source_byte_length: u64,
    graph_id: &'a str,
    candidate_id: &'a str,
    definitions: &'a BTreeMap<String, RoutineDefinition>,
}

#[derive(Serialize)]
struct DefinitionIdentity<'a> {
    definition_id: &'a str,
    node_id: &'a str,
    depends_on: &'a BTreeSet<String>,
    read_sources: &'a [CatalogPath],
    environment: &'a BTreeMap<String, String>,
    timeout_ms: u64,
    output_budget_bytes: u64,
    output_scopes: &'a [CatalogPath],
    primary: &'a RunnerRecipe,
    fallback: &'a Option<RunnerRecipe>,
    runner_policy: &'static str,
    read_policy: &'static str,
}

impl<'a> From<&'a RoutineDefinition> for DefinitionIdentity<'a> {
    fn from(value: &'a RoutineDefinition) -> Self {
        Self {
            definition_id: &value.definition_id,
            node_id: &value.node_id,
            depends_on: &value.depends_on,
            read_sources: &value.read_sources,
            environment: &value.environment,
            timeout_ms: value.timeout_ms,
            output_budget_bytes: value.output_budget_bytes,
            output_scopes: &value.output_scopes,
            primary: &value.primary,
            fallback: &value.fallback,
            runner_policy: RUNNER_POLICY,
            read_policy: READ_POLICY,
        }
    }
}

#[derive(Serialize)]
struct InvocationIdentity<'a> {
    catalog_id: &'a str,
    graph_id: &'a str,
    candidate_id: &'a str,
    plan_id: &'a str,
    definition_id: &'a str,
    definition_sha256: &'a str,
    node_id: &'a str,
    depends_on: &'a [String],
    selected_tool: &'a str,
    selected_tool_identity_sha256: &'a str,
    used_fallback: bool,
    input_id: &'a str,
    program_path_hex: &'a str,
    program_sha256: &'a str,
    program_byte_length: u64,
    program_unix_mode: u32,
    arguments: &'a [String],
    environment: &'a BTreeMap<String, String>,
    read_sources: &'a [BoundReadSource],
    timeout_ms: u64,
    output_budget_bytes: u64,
    output_scopes: &'a [BoundOutputScope],
    runner_policy: &'static str,
    read_policy: &'static str,
}

impl<'a> From<&'a BoundCatalogInvocation> for InvocationIdentity<'a> {
    fn from(value: &'a BoundCatalogInvocation) -> Self {
        Self {
            catalog_id: &value.catalog_id,
            graph_id: &value.graph_id,
            candidate_id: &value.candidate_id,
            plan_id: &value.plan_id,
            definition_id: &value.definition_id,
            definition_sha256: &value.definition_sha256,
            node_id: &value.node_id,
            depends_on: &value.depends_on,
            selected_tool: &value.selected_tool,
            selected_tool_identity_sha256: &value.selected_tool_identity_sha256,
            used_fallback: value.used_fallback,
            input_id: &value.input_id,
            program_path_hex: &value.program_path_hex,
            program_sha256: &value.program_sha256,
            program_byte_length: value.program_byte_length,
            program_unix_mode: value.program_unix_mode,
            arguments: &value.arguments,
            environment: &value.environment,
            read_sources: &value.read_sources,
            timeout_ms: value.timeout_ms,
            output_budget_bytes: value.output_budget_bytes,
            output_scopes: &value.output_scopes,
            runner_policy: value.runner_policy,
            read_policy: value.read_policy,
        }
    }
}

#[derive(Serialize)]
struct InvocationSetIdentity<'a> {
    catalog_id: &'a str,
    graph_id: &'a str,
    candidate_id: &'a str,
    plan_id: &'a str,
    invocation_ids: Vec<&'a str>,
}

fn identifier(value: String) -> CatalogResult<String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(error("catalog-semantic-identifier-invalid"));
    }
    Ok(value)
}

fn identifier_set(
    values: impl IntoIterator<Item = String>,
    duplicate_error: &'static str,
) -> CatalogResult<BTreeSet<String>> {
    let mut result = BTreeSet::new();
    let mut case_values = BTreeSet::new();
    for value in values {
        let value = identifier(value)?;
        if !case_values.insert(value.to_ascii_lowercase()) {
            return Err(error(duplicate_error));
        }
        if !result.insert(value) {
            return Err(error(duplicate_error));
        }
    }
    Ok(result)
}

fn required_sha256(value: String, error_code: &'static str) -> CatalogResult<String> {
    if value.len() != 71
        || !value.starts_with("sha256:")
        || !value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(error(error_code));
    }
    Ok(value)
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn digest_json<T: Serialize + ?Sized>(value: &T) -> CatalogResult<String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| error("catalog-identity-serialization-failed"))
}

fn hex_path(path: &Path) -> CatalogResult<String> {
    let bytes = path
        .to_str()
        .ok_or_else(|| error("catalog-runner-path-invalid"))?
        .as_bytes();
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn error(code: &'static str) -> RoutineCatalogError {
    RoutineCatalogError::new(code)
}
