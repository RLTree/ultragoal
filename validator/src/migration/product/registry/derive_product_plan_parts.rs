fn derive_product_plan_parts(
    input: &ProductInputSnapshot,
) -> Result<ProductPlanParts, ProductMigrationError> {
    input.validate()?;
    let registry = parse_adopted_registry(input.registry().bytes())?;

    let surfaces = input
        .inventory()
        .surfaces
        .iter()
        .map(|surface| (surface.stable_id.as_str(), surface))
        .collect::<BTreeMap<_, _>>();
    reject_active_duplicate_authority(input.inventory().surfaces())?;

    let mut seen_route_ids = BTreeSet::new();
    let mut source_route = BTreeMap::<String, String>::new();
    let mut items = Vec::new();
    let mut effects = Vec::new();

    for route in &registry.routes {
        if !seen_route_ids.insert(route.route_id.clone()) {
            return Err(ProductMigrationError::new(
                "migration-product-duplicate-route",
            ));
        }
        let canonical = surfaces
            .get(route.canonical_target.as_str())
            .copied()
            .ok_or_else(|| ProductMigrationError::new("migration-product-route-target-unknown"))?;
        let matches = input
            .inventory()
            .surfaces
            .iter()
            .filter(|surface| route.match_spec.matches(surface))
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return Err(ProductMigrationError::new(
                "migration-product-route-source-unknown",
            ));
        }
        if route.match_spec.requires_exact_one() && matches.len() != 1 {
            return Err(ProductMigrationError::new(
                "migration-product-route-source-ambiguous",
            ));
        }
        for source in &matches {
            let source = *source;
            if source.stable_id == canonical.stable_id {
                return Err(ProductMigrationError::new(
                    "migration-product-route-self-target",
                ));
            }
            if source_route
                .insert(source.stable_id.clone(), route.route_id.clone())
                .is_some()
            {
                return Err(ProductMigrationError::new(
                    "migration-product-route-source-ambiguous",
                ));
            }
            let semantic_key = format!(
                "migration-route/{}/source/{}",
                route.route_id, source.stable_id
            );
            match &route.transition.adopted_effect {
                None => {
                    if source.status == SurfaceStatus::Active
                        && canonical.status == SurfaceStatus::Active
                    {
                        return Err(ProductMigrationError::new(
                            "migration-product-active-parallel-authority",
                        ));
                    }
                    let reason = if matches.len() == 1
                        && source.status == SurfaceStatus::Active
                        && canonical.status == SurfaceStatus::Definition
                    {
                        "sole_legacy_plus_definition_pending_migration"
                    } else {
                        "registry_transition_not_explicitly_adopted"
                    };
                    items.push(ProductPlanItem::issue(ProductPlanItemDefinition {
                        route_id: route.route_id.clone(),
                        source,
                        canonical_target_id: route.canonical_target.clone(),
                        disposition: PlanDisposition::PendingMigration,
                        effect_id: None,
                        reason,
                    }));
                }
                Some(adoption) => {
                    let disposition = validate_adoption(route, source, canonical, adoption)?;
                    let after_status = parse_post_status(&adoption.post_status)?;
                    let before = AuthoritySnapshot::from_surface(source);
                    let after =
                        AuthoritySnapshot::adopted_postcondition(AdoptedAuthorityPostcondition {
                            source,
                            status: after_status,
                            active_readers: adoption.exact_active_readers.clone(),
                            active_writers: adoption.exact_active_writers.clone(),
                            public_routes: adoption.exact_public_routes.clone(),
                            generated_outputs: adoption.exact_generated_outputs.clone(),
                        });
                    let transition_bytes = serde_json::to_vec(adoption).map_err(|_| {
                        ProductMigrationError::new("migration-product-adopted-transition-invalid")
                    })?;
                    let effect = PlannedMigrationEffect::issue(PlannedMigrationEffectDefinition {
                        semantic_key,
                        route_id: route.route_id.clone(),
                        canonical_target_id: route.canonical_target.clone(),
                        disposition,
                        before,
                        after,
                        adopted_transition_sha256: digest(&transition_bytes),
                        behavior_execution_sha256: adoption.behavior_execution_sha256.clone(),
                        rollback_execution_sha256: adoption.rollback_execution_sha256.clone(),
                        false_pass_control_sha256: adoption.false_pass_control_sha256.clone(),
                        compatibility_prerequisites: adoption.compatibility_prerequisites.clone(),
                    })?;
                    items.push(ProductPlanItem::issue(ProductPlanItemDefinition {
                        route_id: route.route_id.clone(),
                        source,
                        canonical_target_id: route.canonical_target.clone(),
                        disposition,
                        effect_id: Some(effect.effect_id().to_owned()),
                        reason: "exact_adopted_transition",
                    }));
                    effects.push(effect);
                }
            }
        }
    }

    reject_terminal_duplicate_authority(input.inventory().surfaces(), &effects)?;

    Ok(ProductPlanParts {
        input_binding: input.binding(),
        contract_id: registry.contract_id,
        items,
        effects,
    })
}

#[cfg(test)]
pub(crate) fn validate_adopted_registry_bytes(
    bytes: &[u8],
) -> Result<String, ProductMigrationError> {
    Ok(parse_adopted_registry(bytes)?.contract_id)
}

fn parse_adopted_registry(bytes: &[u8]) -> Result<AuthorityRoutingRegistry, ProductMigrationError> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_MIGRATION_REGISTRY_BYTES {
        return Err(ProductMigrationError::new(
            "migration-product-registry-size-refused",
        ));
    }
    let registry: AuthorityRoutingRegistry = serde_json::from_slice(bytes)
        .map_err(|_| ProductMigrationError::new("migration-product-registry-invalid"))?;
    validate_registry(&registry)?;
    Ok(registry)
}
