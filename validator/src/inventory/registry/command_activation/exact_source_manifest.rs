fn exact_source_manifest<'a>(
    actual: impl IntoIterator<Item = &'a WitnessSource>,
) -> Result<BTreeMap<&'a str, &'a WitnessSource>, InventoryError> {
    let expected = WITNESS_SOURCES
        .iter()
        .map(|source| source.relative)
        .collect::<BTreeSet<_>>();
    let mut observed = BTreeMap::new();
    for source in actual {
        if !expected.contains(source.relative) {
            return Err(InventoryError::Activation(ActivationFailure::UnknownRow));
        }
        if source.responsibility.trim().is_empty() {
            return Err(InventoryError::Activation(
                ActivationFailure::ConflictingRow,
            ));
        }
        if let Some(existing) = observed.insert(source.relative, source) {
            return Err(InventoryError::Activation(
                if existing.embedded == source.embedded
                    && existing.responsibility == source.responsibility
                {
                    ActivationFailure::DuplicateRow
                } else {
                    ActivationFailure::ConflictingRow
                },
            ));
        }
    }
    if observed.len() != expected.len()
        || expected
            .iter()
            .any(|relative| !observed.contains_key(relative))
    {
        return Err(InventoryError::Activation(ActivationFailure::MissingRow));
    }
    Ok(observed)
}

fn witness_sources_current(reads: &ReadSession, root: &Path) -> Result<bool, InventoryError> {
    let guard_path = root.join("validator/src/inventory/registry/command_activation/mod.rs");
    if fs::symlink_metadata(guard_path).is_err() {
        return Ok(false);
    }
    let sources = exact_source_manifest(WITNESS_SOURCES)?;
    let present = sources
        .keys()
        .filter(|relative| fs::symlink_metadata(root.join(relative)).is_ok())
        .count();
    if present != sources.len() {
        return Err(InventoryError::Activation(ActivationFailure::MissingRow));
    }
    for source in sources.values() {
        let path = root.join(source.relative);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| InventoryError::Activation(ActivationFailure::MissingRow))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(InventoryError::Activation(ActivationFailure::UnsafeInput));
        }
        let bytes = reads
            .read_bounded(&path, MAX_WITNESS_SOURCE_BYTES)
            .map_err(|error| {
                InventoryError::Activation(
                    if matches!(error, ContextError::ConcurrentMutation(_)) {
                        ActivationFailure::StaleInput
                    } else {
                        ActivationFailure::UnsafeInput
                    },
                )
            })?;
        if bytes.as_slice() != source.embedded {
            return Err(InventoryError::Activation(ActivationFailure::StaleInput));
        }
    }
    Ok(true)
}

pub(crate) fn revalidate_sources(
    reads: &ReadSession,
    root: &Path,
    expected_current: bool,
) -> Result<(), InventoryError> {
    let current = witness_sources_current(reads, root)?;
    if current != expected_current {
        return Err(InventoryError::Activation(ActivationFailure::StaleInput));
    }
    reads
        .revalidate()
        .map_err(|_| InventoryError::Activation(ActivationFailure::StaleInput))
}

fn api_rows() -> Result<BTreeMap<String, ActivationRow>, InventoryError> {
    let mut rows = BTreeMap::new();
    for api in crate::api_witness::implemented_public_apis() {
        let stable_id = format!("API:{api}");
        let row = ActivationRow {
            stable_id: stable_id.clone(),
            digest_sha256: sha256_hex(api.as_bytes()),
            active_status: ActiveStatus::Active,
            references: Vec::new(),
        };
        if rows.insert(stable_id, row).is_some() {
            return Err(InventoryError::Activation(ActivationFailure::DuplicateRow));
        }
    }
    Ok(rows)
}

fn command_rows() -> Result<BTreeMap<String, ActivationRow>, InventoryError> {
    let mut rows = BTreeMap::new();
    for group in crate::command_witness::compiled_groups()
        .map_err(|_| InventoryError::Activation(ActivationFailure::ConflictingRow))?
    {
        let stable_id = format!("COMMAND:{}", group.name);
        let row = ActivationRow {
            stable_id: stable_id.clone(),
            digest_sha256: group.digest_sha256,
            active_status: ActiveStatus::Candidate,
            references: vec![
                "ACTIVATION-CLASS:catalog-definition-only".to_owned(),
                format!("ROUTE-COUNT:{}", group.route_count),
            ],
        };
        if rows.insert(stable_id, row).is_some() {
            return Err(InventoryError::Activation(ActivationFailure::DuplicateRow));
        }
    }
    Ok(rows)
}
