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
    let mut node_case_ids = BTreeSet::new();
    for raw_definition in raw.routines {
        let node_id = identifier(raw_definition.node_id)?;
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
        if raw_definition.behavior_id != ROUTINE_BEHAVIOR {
            return Err(error("catalog-behavior-unsupported"));
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
        if raw_definition.timeout_ms == 0 || raw_definition.timeout_ms > MAX_TIMEOUT_MS {
            return Err(error("catalog-timeout-invalid"));
        }
        if raw_definition.output_budget_bytes == 0
            || raw_definition.output_budget_bytes > MAX_OUTPUT_BUDGET_BYTES
        {
            return Err(error("catalog-output-budget-invalid"));
        }
        let mut definition = RoutineDefinition {
            definition_id: format!("routine.{node_id}.v2"),
            definition_sha256: String::new(),
            node_id: node_id.clone(),
            behavior_id: raw_definition.behavior_id,
            depends_on,
            read_sources,
            timeout_ms: raw_definition.timeout_ms,
            output_budget_bytes: raw_definition.output_budget_bytes,
            output_scopes,
        };
        definition.definition_sha256 = digest_json(&DefinitionIdentity::from(&definition))?;
        definitions.insert(node_id, definition);
    }
    if definitions.keys().ne(adoption.nodes.keys()) {
        return Err(error("catalog-definition-set-inexact"));
    }
    validate_global_path_compatibility(&definitions)?;
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
