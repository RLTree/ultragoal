impl RegistryMatch {
    fn validate(&self) -> Result<(), ProductMigrationError> {
        if self.stable_id.is_none() && self.kind.is_none() {
            return Err(ProductMigrationError::new(
                "migration-product-route-match-invalid",
            ));
        }
        if self
            .stable_id
            .as_deref()
            .is_some_and(|value| !valid_stable_identifier(value))
            || self
                .kind
                .as_deref()
                .is_some_and(|value| !valid_identifier(value))
            || self
                .relative_path
                .as_deref()
                .is_some_and(|value| !value.is_ascii() || !super::super::safe_relative_path(value))
        {
            return Err(ProductMigrationError::new(
                "migration-product-route-match-invalid",
            ));
        }
        Ok(())
    }

    fn matches(&self, surface: &InventorySurface) -> bool {
        self.stable_id
            .as_deref()
            .is_none_or(|value| value == surface.stable_id)
            && self
                .kind
                .as_deref()
                .is_none_or(|value| value == surface.kind)
            && self
                .relative_path
                .as_deref()
                .is_none_or(|value| value == surface.relative_path)
    }

    fn requires_exact_one(&self) -> bool {
        self.stable_id.is_some() || self.relative_path.is_some()
    }
}

fn validate_registry(registry: &AuthorityRoutingRegistry) -> Result<(), ProductMigrationError> {
    if registry.schema_version != REGISTRY_SCHEMA
        || !valid_identifier(&registry.contract_id)
        || registry.destructive_cleanup_authorized
        || registry.authority_rule.is_empty()
        || registry.authority_rule.len() > 4_096
        || registry.authority_rule.chars().any(char::is_control)
        || registry.routes.len() > super::model::MAX_PRODUCT_ITEMS
    {
        return Err(ProductMigrationError::new(
            "migration-product-registry-contract-refused",
        ));
    }
    for route in &registry.routes {
        if !valid_identifier(&route.route_id)
            || !valid_stable_identifier(&route.canonical_target)
            || route.intended_disposition != "non-authoritative"
            || crate::inventory::behavioral_role::legacy_route_targets_active_role(
                route.match_spec.stable_id.as_deref(),
                route.match_spec.relative_path.as_deref(),
            )
        {
            return Err(ProductMigrationError::new(
                "migration-product-route-invalid",
            ));
        }
        route.match_spec.validate()?;
        validate_transition_shape(&route.transition)?;
    }
    Ok(())
}

fn validate_transition_shape(transition: &RegistryTransition) -> Result<(), ProductMigrationError> {
    let known = [
        matches!(
            transition.compatibility_behavior.as_str(),
            "unverified" | "exact-route-only" | "not-applicable" | "removed"
        ),
        matches!(
            transition.compatibility_boundary.as_str(),
            "blocked-by-OD-008" | "explicit-only" | "adopted" | "closed"
        ),
        matches!(
            transition.replacement_state.as_str(),
            "unverified" | "candidate-required" | "verified"
        ),
        matches!(
            transition.active_reader_writer_state.as_str(),
            "active" | "none" | "none-verified"
        ),
        matches!(
            transition.observed_authority_state.as_str(),
            "active" | "compatibility-route-retained" | "context-only" | "retired"
        ),
        matches!(
            transition.equivalence_proof.as_str(),
            "missing" | "not-applicable" | "executed-behavior-v1"
        ),
        matches!(
            transition.physical_cleanup_state.as_str(),
            "blocked-by-OD-009" | "preserve"
        ),
    ];
    let proof_refs = transition.proof_refs.iter().collect::<BTreeSet<_>>();
    if known.into_iter().any(|value| !value)
        || transition.proof_refs.len() > 4_096
        || proof_refs.len() != transition.proof_refs.len()
        || transition
            .proof_refs
            .iter()
            .any(|value| !safe_reference(value))
    {
        return Err(ProductMigrationError::new(
            "migration-product-transition-shape-refused",
        ));
    }
    Ok(())
}
