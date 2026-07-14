use super::*;

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
