fn verify_artifact_against_source(
    artifact: &ProductionPackageArtifact,
    context: &LiveContext,
    catalog: &AuthorityCatalog,
    source: &SourcePackageSnapshot,
) -> Result<(), ProductionPackageError> {
    validate_context_catalog(context, catalog)?;
    let current_candidate = candidate_id(context)?;
    if artifact.context_id != context.context_id()
        || artifact.candidate_id != current_candidate
        || artifact.catalog_id != catalog.catalog_id()
    {
        return Err(failure(ProductionPackageErrorId::CatalogMismatch));
    }

    let decoded = archive::decode(artifact.snapshot.archive())
        .map_err(|_| failure(ProductionPackageErrorId::ArchiveMismatch))?;
    manifest::validate(&decoded.entries, &decoded.plugin_id, &decoded.version)
        .map_err(|_| failure(ProductionPackageErrorId::ManifestMismatch))?;
    let (inventory, inventory_sha256) = archive::inventory(&decoded)
        .map_err(|_| failure(ProductionPackageErrorId::InventoryMismatch))?;
    if inventory != artifact.snapshot.inventory()
        || inventory_sha256 != artifact.snapshot.inventory_sha256()
        || entry_tree_sha256(&decoded.entries)
            .map_err(|_| failure(ProductionPackageErrorId::InventoryMismatch))?
            != decoded.source_tree_sha256
    {
        return Err(failure(ProductionPackageErrorId::InventoryMismatch));
    }

    let fresh = build_artifact(
        context,
        catalog.catalog_id(),
        source,
        artifact.cli_payload.clone(),
    )?;
    if !same_artifact(artifact, &fresh) {
        return Err(failure(ProductionPackageErrorId::SourceUnavailable));
    }
    validate_context_catalog(context, catalog)
}

fn same_artifact(expected: &ProductionPackageArtifact, actual: &ProductionPackageArtifact) -> bool {
    expected.context_id == actual.context_id
        && expected.candidate_id == actual.candidate_id
        && expected.catalog_id == actual.catalog_id
        && expected.source_snapshot_id == actual.source_snapshot_id
        && expected.source_inventory == actual.source_inventory
        && expected.cli_payload == actual.cli_payload
        && expected.plan.context_id == actual.plan.context_id
        && expected.plan.candidate_id == actual.plan.candidate_id
        && expected.plan.plugin_id == actual.plan.plugin_id
        && expected.plan.version == actual.plan.version
        && expected.plan.source_date_epoch == actual.plan.source_date_epoch
        && expected.plan.catalog_id == actual.plan.catalog_id
        && expected.plan.accepted_inventory_sha256 == actual.plan.accepted_inventory_sha256
        && expected.plan.source_tree_sha256 == actual.plan.source_tree_sha256
        && expected.plan.entries == actual.plan.entries
        && expected.snapshot == actual.snapshot
        && expected.binding == actual.binding
}

fn validate_context_catalog(
    context: &LiveContext,
    catalog: &AuthorityCatalog,
) -> Result<(), ProductionPackageError> {
    context
        .revalidate()
        .map_err(|_| failure(ProductionPackageErrorId::ContextUnavailable))?;
    if catalog.context_id() != context.context_id() || catalog.revalidate_identity().is_err() {
        return Err(failure(ProductionPackageErrorId::CatalogMismatch));
    }
    Ok(())
}

fn build_artifact(
    context: &LiveContext,
    catalog_id: &str,
    source: &SourcePackageSnapshot,
    cli_payload: Option<CandidateCliPayload>,
) -> Result<ProductionPackageArtifact, ProductionPackageError> {
    if source.context_id() != context.context_id() {
        return Err(failure(ProductionPackageErrorId::ContextUnavailable));
    }
    let version = validate_manifests(source)?;
    let source_entries = packaged_entries(source, None)?;
    let entries = packaged_entries(source, cli_payload.as_ref())?;
    manifest::validate(&entries, PLUGIN_ID, &version)
        .map_err(|_| failure(ProductionPackageErrorId::ManifestMismatch))?;
    let candidate_id = candidate_id(context)?;
    if cli_payload
        .as_ref()
        .is_some_and(|payload| payload.candidate_id() != candidate_id)
    {
        return Err(failure(ProductionPackageErrorId::CatalogMismatch));
    }
    let source_tree_sha256 = entry_tree_sha256(&entries)
        .map_err(|_| failure(ProductionPackageErrorId::InventoryMismatch))?;
    let source_inventory = source_inventory(
        context.context_id(),
        &candidate_id,
        catalog_id,
        &version,
        &source_entries,
    )?;
    let plan = PackagePlan {
        context_id: context.context_id().to_owned(),
        candidate_id: candidate_id.clone(),
        plugin_id: PLUGIN_ID.to_owned(),
        version,
        source_date_epoch: 0,
        catalog_id: catalog_id.to_owned(),
        accepted_inventory_sha256: sha256(&source_inventory),
        source_tree_sha256,
        entries,
    };
    let archive =
        archive::encode(&plan).map_err(|_| failure(ProductionPackageErrorId::ArchiveMismatch))?;
    let snapshot = verify_package(&plan, &archive)
        .map_err(|_| failure(ProductionPackageErrorId::ArchiveMismatch))?;
    let binding = PackageArtifactBinding::issue(&snapshot)
        .map_err(|_| failure(ProductionPackageErrorId::ArchiveMismatch))?;
    Ok(ProductionPackageArtifact {
        context_id: context.context_id().to_owned(),
        candidate_id,
        catalog_id: catalog_id.to_owned(),
        source_snapshot_id: source.snapshot_id().to_owned(),
        source_inventory,
        cli_payload,
        plan,
        snapshot,
        binding,
    })
}

fn validate_manifests(source: &SourcePackageSnapshot) -> Result<String, ProductionPackageError> {
    let supported_bytes = source
        .bytes(SUPPORTED_MANIFEST_PATH)
        .ok_or_else(|| failure(ProductionPackageErrorId::ManifestMismatch))?;
    let supported = plugin_manifest::parse(supported_bytes, plugin_manifest::MANIFEST_LIMIT)
        .map_err(|_| failure(ProductionPackageErrorId::ManifestMismatch))?;
    if !plugin_manifest::semantic_issues(&supported).is_empty()
        || supported.name != PLUGIN_ID
        || supported.version != SUPPORTED_VERSION
        || supported.skills.as_deref() != Some("./skills/")
    {
        return Err(failure(ProductionPackageErrorId::ManifestMismatch));
    }
    let draft = source.manifest();
    if draft.name() != PLUGIN_ID || draft.version() != SUPPORTED_VERSION {
        return Err(failure(ProductionPackageErrorId::ManifestMismatch));
    }
    let skills = draft.skills();
    if skills.len() != CANONICAL_SKILLS.len()
        || skills.iter().zip(CANONICAL_SKILLS).any(|(row, name)| {
            let expected_path = format!("skills/{name}/SKILL.md");
            row.name() != name || row.path() != expected_path
        })
    {
        return Err(failure(ProductionPackageErrorId::ManifestMismatch));
    }
    Ok(SUPPORTED_VERSION.to_owned())
}
