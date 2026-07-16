use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct BoundCatalogInvocation {
    pub(crate) invocation_id: String,
    pub(crate) catalog_id: String,
    pub(crate) graph_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plan_id: String,
    pub(crate) definition_id: String,
    pub(crate) definition_sha256: String,
    pub(crate) node_id: String,
    pub(crate) behavior_id: String,
    pub(crate) depends_on: Vec<String>,
    pub(crate) selected_tool: String,
    pub(crate) selected_tool_identity_sha256: String,
    pub(crate) used_fallback: bool,
    pub(crate) input_id: String,
    pub(crate) program_path_hex: String,
    pub(crate) program_sha256: String,
    pub(crate) program_byte_length: u64,
    pub(crate) program_unix_mode: u32,
    pub(crate) arguments: Vec<String>,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) read_sources: Vec<BoundReadSource>,
    pub(crate) timeout_ms: u64,
    pub(crate) output_budget_bytes: u64,
    pub(crate) output_scopes: Vec<BoundOutputScope>,
    pub(crate) runner_policy: &'static str,
    pub(crate) read_policy: &'static str,
}

impl BoundCatalogInvocation {
    pub(crate) fn node_id(&self) -> &str {
        &self.node_id
    }

    pub(crate) fn behavior_id(&self) -> &str {
        &self.behavior_id
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
    pub(crate) invocation_set_id: String,
    pub(crate) catalog_id: String,
    pub(crate) graph_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plan_id: String,
    pub(crate) invocations: Vec<BoundCatalogInvocation>,
}

impl BoundRoutineInvocationSet {
    #[cfg(test)]
    pub(crate) fn invocation_set_id(&self) -> &str {
        &self.invocation_set_id
    }

    pub(crate) fn invocations(&self) -> &[BoundCatalogInvocation] {
        &self.invocations
    }
}

#[derive(Debug)]
pub(crate) struct ProductionRoutineCatalog {
    pub(crate) source: SealedFile,
    pub(crate) catalog_id: String,
    pub(crate) graph_id: String,
    pub(crate) candidate_id: String,
    pub(crate) definitions: BTreeMap<String, RoutineDefinition>,
}
