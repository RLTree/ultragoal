fn product_plan_digest(
    binding: &MigrationInputBinding,
    contract_id: &str,
    compatibility_boundary_binding: &Option<CompatibilityBoundaryBinding>,
    items: &[ProductPlanItem],
    effects: &[PlannedMigrationEffect],
) -> Result<String, ProductMigrationError> {
    let rows = serde_json::to_vec(&(compatibility_boundary_binding, items, effects))
        .map_err(|_| ProductMigrationError::new("migration-product-plan-output-invalid"))?;
    if rows.len() > MAX_MACHINE_OUTPUT_BYTES {
        return Err(ProductMigrationError::new(
            "migration-product-plan-output-too-large",
        ));
    }
    Ok(digest(
        format!(
            "migration-product-plan-v2|{}|{}|{}",
            binding.binding_sha256,
            contract_id,
            digest(&rows),
        )
        .as_bytes(),
    ))
}

fn validate_inventory_paths(inventory: &MigrationInventory) -> Result<(), ProductMigrationError> {
    let mut exact_paths = BTreeSet::new();
    let mut casefold_paths = BTreeMap::<String, String>::new();
    for surface in &inventory.surfaces {
        if !surface.relative_path.is_ascii() || !safe_relative_path(&surface.relative_path) {
            return Err(ProductMigrationError::new(
                "migration-product-inventory-unicode-or-path-refused",
            ));
        }
        if !matches!(
            (surface.file_kind, surface.link_count),
            (SurfaceFileKind::Regular, 1) | (SurfaceFileKind::Semantic, 0)
        ) {
            return Err(ProductMigrationError::new(
                "migration-product-inventory-file-refused",
            ));
        }
        if !exact_paths.insert(surface.relative_path.clone()) {
            return Err(ProductMigrationError::new(
                "migration-product-inventory-path-alias",
            ));
        }
        let folded = surface.relative_path.to_ascii_lowercase();
        if casefold_paths
            .insert(folded, surface.relative_path.clone())
            .is_some()
        {
            return Err(ProductMigrationError::new(
                "migration-product-inventory-path-collision",
            ));
        }
    }
    Ok(())
}

pub(super) fn exact_input_matches(
    binding: &MigrationInputBinding,
    input: &ProductInputSnapshot,
) -> bool {
    binding.validate() && binding == &MigrationInputBinding::issue(input)
}
