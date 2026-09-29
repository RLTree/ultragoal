#[derive(Clone, Debug, Eq, PartialEq)]
struct ActivationRow {
    stable_id: String,
    digest_sha256: String,
    active_status: ActiveStatus,
    references: Vec<String>,
}

impl ActivationRow {
    fn from_entry(entry: &InventoryEntry) -> Self {
        Self {
            stable_id: entry.stable_id.clone(),
            digest_sha256: entry.digest_sha256.clone(),
            active_status: entry.active_status,
            references: entry.references.clone(),
        }
    }
}

fn exact_rows(
    actual: impl IntoIterator<Item = ActivationRow>,
    expected: &BTreeMap<String, ActivationRow>,
) -> Result<(), InventoryError> {
    let mut observed = BTreeMap::new();
    for row in actual {
        if !expected.contains_key(&row.stable_id) {
            return Err(InventoryError::Activation(ActivationFailure::UnknownRow));
        }
        if let Some(existing) = observed.insert(row.stable_id.clone(), row.clone()) {
            return Err(InventoryError::Activation(if existing == row {
                ActivationFailure::DuplicateRow
            } else {
                ActivationFailure::ConflictingRow
            }));
        }
    }
    if observed.len() != expected.len()
        || expected
            .keys()
            .any(|stable_id| !observed.contains_key(stable_id))
    {
        return Err(InventoryError::Activation(ActivationFailure::MissingRow));
    }
    if expected
        .iter()
        .any(|(stable_id, expected)| observed.get(stable_id) != Some(expected))
    {
        return Err(InventoryError::Activation(
            ActivationFailure::ConflictingRow,
        ));
    }
    Ok(())
}

fn exact_set(
    actual: impl IntoIterator<Item = String>,
    expected: &BTreeSet<String>,
) -> Result<(), InventoryError> {
    let mut observed = BTreeSet::new();
    for stable_id in actual {
        if !expected.contains(&stable_id) {
            return Err(InventoryError::Activation(ActivationFailure::UnknownRow));
        }
        if !observed.insert(stable_id) {
            return Err(InventoryError::Activation(ActivationFailure::DuplicateRow));
        }
    }
    if &observed != expected {
        return Err(InventoryError::Activation(ActivationFailure::MissingRow));
    }
    Ok(())
}
