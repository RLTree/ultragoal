// This is the direct source closure for API and command activation. It binds
// declaration, registry construction, exact-row gating, status finalization,
// public serialization, and the production summary export. Generic context and
// confined-I/O primitives remain covered by the accepted context dependency.
const WITNESS_SOURCES: &[WitnessSource] = &[
    source!(
        "validator/src/api_witness.rs",
        include_bytes!("../../../api_witness.rs"),
        "compiled API declaration"
    ),
    source!(
        "validator/src/lib.rs",
        include_bytes!("../../../lib.rs"),
        "root public module export authority"
    ),
    source!(
        "validator/tests/public_api_witness.rs",
        include_bytes!("../../../../tests/public_api_witness.rs"),
        "external crate visibility witness"
    ),
    source!(
        "validator/src/command_witness.rs",
        include_bytes!("../../../command_witness.rs"),
        "compiled command row digest"
    ),
    source!(
        "validator/src/cli/successor/catalog/mod.rs",
        include_bytes!("../../../cli/successor/catalog/mod.rs"),
        "command catalog composition"
    ),
    source!(
        "validator/src/cli/successor/catalog/evaluation_and_migration.rs",
        include_bytes!("../../../cli/successor/catalog/evaluation_and_migration.rs"),
        "evaluation and migration command rows"
    ),
    source!(
        "validator/src/cli/successor/catalog/repository_fit_and_checks.rs",
        include_bytes!("../../../cli/successor/catalog/repository_fit_and_checks.rs"),
        "repository fit and validation command rows"
    ),
    source!(
        "validator/src/cli/successor/catalog/inspection.rs",
        include_bytes!("../../../cli/successor/catalog/inspection.rs"),
        "inspection command rows"
    ),
    source!(
        "validator/src/cli/successor/catalog/observability_and_package.rs",
        include_bytes!("../../../cli/successor/catalog/observability_and_package.rs"),
        "observability and package command rows"
    ),
    source!(
        "validator/src/cli/successor/catalog/options.rs",
        include_bytes!("../../../cli/successor/catalog/options.rs"),
        "command option contracts"
    ),
    source!(
        "validator/src/cli/successor/command_contract/mod.rs",
        include_bytes!("../../../cli/successor/command_contract/mod.rs"),
        "command model composition"
    ),
    source!(
        "validator/src/cli/successor/command_contract/arguments.rs",
        include_bytes!("../../../cli/successor/command_contract/arguments.rs"),
        "typed command arguments"
    ),
    source!(
        "validator/src/cli/successor/command_contract/commands.rs",
        include_bytes!("../../../cli/successor/command_contract/commands.rs"),
        "command group and action model"
    ),
    source!(
        "validator/src/cli/successor/command_contract/descriptor.rs",
        include_bytes!("../../../cli/successor/command_contract/descriptor.rs"),
        "command descriptor model"
    ),
    source!(
        "validator/src/cli/successor/command_contract/exit.rs",
        include_bytes!("../../../cli/successor/command_contract/exit.rs"),
        "command effect and exit model"
    ),
    source!(
        "validator/src/cli/successor/command_contract/invocation.rs",
        include_bytes!("../../../cli/successor/command_contract/invocation.rs"),
        "parsed invocation model"
    ),
    source!(
        "validator/src/inventory/mod.rs",
        include_bytes!("../../mod.rs"),
        "public inventory export"
    ),
    source!(
        "validator/src/inventory/builder.rs",
        include_bytes!("../../builder.rs"),
        "guard ordering and catalog finalization"
    ),
    source!(
        "validator/src/inventory/digest.rs",
        include_bytes!("../../digest.rs"),
        "activation row digest"
    ),
    source!(
        "validator/src/inventory/fs.rs",
        include_bytes!("../../fs.rs"),
        "registry source read gate"
    ),
    source!(
        "validator/src/inventory/projection.rs",
        include_bytes!("../../projection.rs"),
        "catalog projection export"
    ),
    source!(
        "validator/src/inventory/registry/command_activation/mod.rs",
        include_bytes!("mod.rs"),
        "exact activation guard"
    ),
    source!(
        "validator/src/inventory/registry/command_activation/guard.rs",
        include_bytes!("guard.rs"),
        "frontier-aware activation decision"
    ),
    source!(
        "validator/src/inventory/registry/command_activation/exact_source_manifest.rs",
        include_bytes!("exact_source_manifest.rs"),
        "activation source and row verifier"
    ),
    source!(
        "validator/src/inventory/registry/data.rs",
        include_bytes!("../data.rs"),
        "registry identifier and row storage"
    ),
    source!(
        "validator/src/inventory/registry/integrity.rs",
        include_bytes!("../integrity.rs"),
        "adopted registry integrity gate"
    ),
    registry_source!("frontier.rs", "adopted dependency frontier"),
    registry_source!("load.rs", "frontier-bound registry construction"),
    registry_source!("mod.rs", "registry load and guard export"),
    registry_source!("semantic.rs", "active API row construction"),
    source!(
        "validator/src/inventory/registry/sources.rs",
        include_bytes!("../sources.rs"),
        "registry source coverage gate"
    ),
    source!(
        "validator/src/inventory/registry/topology.rs",
        include_bytes!("../topology.rs"),
        "candidate command row construction"
    ),
    source!(
        "validator/src/inventory/types/mod.rs",
        include_bytes!("../../types/mod.rs"),
        "active status and catalog serialization"
    ),
    source!(
        "validator/src/inventory/validate/duplicates.rs",
        include_bytes!("../../validate/duplicates.rs"),
        "duplicate activation classification"
    ),
    source!(
        "validator/src/inventory/validate/mod.rs",
        include_bytes!("../../validate/mod.rs"),
        "catalog status reconciliation"
    ),
    source!(
        "validator/examples/hct_inventory.rs",
        include_bytes!("../../../../examples/hct_inventory.rs"),
        "production catalog and closure export"
    ),
];

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
