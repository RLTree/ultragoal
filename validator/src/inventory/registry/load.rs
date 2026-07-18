pub(crate) fn load(
    reads: &ReadSession,
    root: &Path,
    expected_handoff_digest: &str,
) -> Result<RegistryData, InventoryError> {
    integrity::verify(reads, root, expected_handoff_digest)?;
    let specifications = [
        (
            "PRODUCT_SURFACE_INVENTORY.json",
            "surfaces",
            "surface_id",
            "product-surface-definition",
        ),
        (
            "CUSTOM_TOOL_INVENTORY.json",
            "tools",
            "tool_id",
            "custom-tool-definition",
        ),
        (
            "CLAIM_REGISTRY.json",
            "claims",
            "claim_id",
            "claim-definition",
        ),
        (
            "REQUIREMENT_TRACE.json",
            "requirements",
            "requirement_id",
            "requirement-definition",
        ),
    ];
    let mut entries = Vec::new();
    let mut counts = BTreeMap::new();
    let mut findings = Vec::new();
    let mut contract_id = None;
    let mut product = None;
    let mut required_apis: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let frontier = frontier::load(reads, root)?;
    entries.extend(frontier.entries);
    for (name, array, id_key, kind) in specifications {
        let (path, value) = json(reads, root, name)?;
        let observed_contract = value
            .get("contract_id")
            .and_then(Value::as_str)
            .filter(|value| safe_identifier(value))
            .ok_or_else(|| InventoryError::InvalidRegistry("invalid contract_id".to_owned()))?;
        if contract_id
            .as_deref()
            .is_some_and(|known| known != observed_contract)
        {
            findings.push(InventoryFinding::error(
                "contract_id_disagreement",
                None,
                Some(name),
                "contract registry IDs disagree".to_owned(),
            ));
        }
        contract_id.get_or_insert_with(|| observed_contract.to_owned());
        let source_rows = rows(&value, array)?;
        let count_key = match array {
            "surfaces" => "surface_count",
            "tools" => "tool_count",
            "claims" => "claim_count",
            _ => "requirement_count",
        };
        validate_count(&value, count_key, source_rows.len(), array, &mut findings);
        counts.insert(array.to_owned(), source_rows.len());
        entries.push(contract_source_entry(reads, root, &path, name)?);
        for row in source_rows {
            let stable_id = id(row, id_key)?;
            if array == "tools" {
                for api in row
                    .get("public_api")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    if !safe_identifier(api) {
                        return Err(InventoryError::InvalidRegistry(
                            "custom tool has a noncanonical public API identifier".to_owned(),
                        ));
                    }
                    required_apis
                        .entry(api.to_owned())
                        .or_default()
                        .insert(stable_id.clone());
                }
            }
            entries.push(registry_entry(row, stable_id, kind, "OWN-UNKNOWN", name)?);
        }
        if array == "surfaces" {
            product = Some(value);
        }
    }
    counts.insert(
        "contract_sources".to_owned(),
        sources::load(reads, root, &mut entries)?,
    );
    let product = product.expect("surface registry loaded");
    semantic::load(semantic::SemanticRegistryLoad {
        reads,
        root,
        product: &product,
        required_apis,
        active_tools: frontier.active_tools,
        entries: &mut entries,
        counts: &mut counts,
        findings: &mut findings,
    })?;
    let legacy_skills = topology::load(&product, &mut entries, &mut counts)?;
    Ok(RegistryData {
        contract_id: contract_id.unwrap_or_default(),
        counts,
        entries,
        findings,
        legacy_skills,
    })
}
