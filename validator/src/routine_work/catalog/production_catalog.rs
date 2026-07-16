use super::*;

impl ProductionRoutineCatalog {
    pub(crate) fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    #[cfg(test)]
    pub(crate) fn graph_id(&self) -> &str {
        &self.graph_id
    }

    #[cfg(test)]
    pub(crate) fn definition_count(&self) -> usize {
        self.definitions.len()
    }

    #[cfg(test)]
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
        let runner = request
            .runners
            .get(ROUTINE_RUNNER)
            .ok_or_else(|| error("catalog-selection-runner-unobserved"))?;
        let runner_seal = capture_program(runner)?;

        for selected in &request.selected {
            let definition = self
                .definitions
                .get(&selected.node_id)
                .ok_or_else(|| error("catalog-selection-node-unknown"))?;
            if selected.depends_on != definition.depends_on {
                return Err(error("catalog-selection-dependencies-stale"));
            }
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
                validate_output_scope_prefix(&self.source.root, relative)?;
                bound_outputs.push(BoundOutputScope {
                    relative_path: relative.as_str().to_owned(),
                });
            }

            let environment = crate::routine_work::fixed_environment(&runner.executable_path)
                .map_err(|_| error("catalog-runner-parent-invalid"))?;

            let mut invocation = BoundCatalogInvocation {
                invocation_id: String::new(),
                catalog_id: self.catalog_id.clone(),
                graph_id: self.graph_id.clone(),
                candidate_id: self.candidate_id.clone(),
                plan_id: request.plan_id.clone(),
                definition_id: definition.definition_id.clone(),
                definition_sha256: definition.definition_sha256.clone(),
                node_id: definition.node_id.clone(),
                behavior_id: definition.behavior_id.clone(),
                depends_on: definition.depends_on.iter().cloned().collect(),
                selected_tool: ROUTINE_RUNNER.to_owned(),
                selected_tool_identity_sha256: runner.tool_identity_sha256.clone(),
                used_fallback: false,
                input_id: selected.input_id.clone(),
                program_path_hex: hex_path(&runner.executable_path)?,
                program_sha256: runner.program_sha256.clone(),
                program_byte_length: runner.program_byte_length,
                program_unix_mode: runner.program_unix_mode,
                arguments: ROUTINE_ARGUMENTS
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
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
        runner_seal.verify_current(MAX_READ_SOURCE_BYTES)?;

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
