use super::*;

pub(crate) fn validate_raw_catalog(
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

pub(crate) fn validate_global_path_compatibility(
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

pub(crate) fn validate_recipe(
    raw: RawRunnerRecipe,
    expected_tool: &str,
) -> CatalogResult<RunnerRecipe> {
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
