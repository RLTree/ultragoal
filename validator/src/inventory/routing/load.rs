pub(crate) fn load(
    reads: &ReadSession,
    root: &Path,
    contract_id: &str,
) -> Result<RoutingData, InventoryError> {
    let observation = ObservedMigrationRegistry::observe(reads, root)?;
    let registry =
        serde_json::from_slice::<RouteRegistry>(observation.bytes()).map_err(|error| {
            InventoryError::Json {
                path: root.join(MIGRATION_REGISTRY_PATH),
                message: error.to_string(),
            }
        })?;
    validate(&registry, contract_id)?;
    let registry_entry = observation.inventory_entry();
    Ok(RoutingData {
        registry_entry,
        registry,
        observation,
    })
}
