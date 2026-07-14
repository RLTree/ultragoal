pub(crate) fn guard(
    reads: &ReadSession,
    root: &Path,
    registry: &mut RegistryData,
) -> Result<bool, InventoryError> {
    let expected_apis = api_rows()?;
    let expected_commands = command_rows()?;
    let required_apis = registry
        .entries
        .iter()
        .filter(|entry| {
            entry.kind == "source-symbol-implementation"
                && entry.relative_path.starts_with("@semantic/")
        })
        .map(|entry| entry.stable_id.clone())
        .collect::<BTreeSet<_>>();
    if expected_apis
        .keys()
        .any(|stable_id| !required_apis.contains(stable_id))
    {
        return Err(InventoryError::Activation(ActivationFailure::UnknownRow));
    }
    exact_rows(
        registry
            .entries
            .iter()
            .filter(|entry| entry.generator.as_deref() == Some(API_GENERATOR))
            .map(ActivationRow::from_entry),
        &expected_apis,
    )?;
    let required_commands = registry
        .entries
        .iter()
        .filter(|entry| {
            entry.kind == "command-group" && entry.active_status == ActiveStatus::Required
        })
        .map(|entry| entry.stable_id.clone());
    exact_set(
        required_commands,
        &expected_commands.keys().cloned().collect(),
    )?;
    exact_rows(
        registry
            .entries
            .iter()
            .filter(|entry| entry.generator.as_deref() == Some(COMMAND_GENERATOR))
            .map(ActivationRow::from_entry),
        &expected_commands,
    )?;

    let sources_current = witness_sources_current(reads, root)?;
    if !sources_current {
        registry.findings.push(InventoryFinding::warning(
            "activation_witness_source_set_unverified",
            None,
            None,
            "compiled activation witnesses cannot be bound to the target source set".to_owned(),
        ));
    }
    for entry in registry
        .entries
        .iter_mut()
        .filter(|entry| entry.generator.as_deref() == Some(API_GENERATOR))
    {
        if !sources_current {
            entry.active_status = ActiveStatus::Candidate;
        }
        entry.input_provenance.extend(
            WITNESS_SOURCES
                .iter()
                .map(|source| source.relative.to_owned()),
        );
        entry.normalize();
    }
    registry.counts.insert(
        "activation_witness_sources".to_owned(),
        WITNESS_SOURCES.len(),
    );
    registry.counts.insert(
        "verified_api_activations".to_owned(),
        usize::from(sources_current) * expected_apis.len(),
    );
    registry
        .counts
        .insert("verified_command_handler_activations".to_owned(), 0);
    registry.counts.insert(
        "candidate_command_groups".to_owned(),
        expected_commands.len(),
    );
    Ok(sources_current)
}
