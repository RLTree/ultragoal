pub(crate) fn load(
    reads: &ReadSession,
    root: &Path,
    contract_id: &str,
) -> Result<RoutingData, InventoryError> {
    let path = root.join(ROUTES_PATH);
    let metadata = fs::symlink_metadata(&path).map_err(|error| InventoryError::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(invalid("registry must be a regular non-symlink file"));
    }
    let bytes = read_bounded(reads, &path, MAX_ROUTING_BYTES)?;
    let registry =
        serde_json::from_slice::<RouteRegistry>(&bytes).map_err(|error| InventoryError::Json {
            path: path.clone(),
            message: error.to_string(),
        })?;
    validate(&registry, contract_id)?;
    let reader_proof_is_current =
        has_agent_context_routes(&registry) && reader_proof_current(reads, root);
    let archive_proof_is_current =
        has_archive_routes(&registry) && archive_proof_current(reads, root);
    let registry_entry = physical_entry(
        reads,
        root,
        &path,
        PhysicalEntryDescriptor {
            stable_id: "AUTHORITY-ROUTING-REGISTRY".to_owned(),
            kind: "migration-authority-registry",
            owner: "OWN-ULTRA-ROOT",
            authority_state: AuthorityState::Canonical,
            active_status: ActiveStatus::Active,
            generator: None,
            provenance: vec!["docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/MIGRATION-AND-RETIREMENT.md".to_owned()],
            references: Vec::new(),
        },
    )?;
    Ok(RoutingData {
        registry_entry,
        registry,
        reader_proof_is_current,
        archive_proof_is_current,
    })
}
