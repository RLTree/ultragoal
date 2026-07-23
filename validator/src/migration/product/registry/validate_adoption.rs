fn validate_adoption(
    route: &RegistryRoute,
    source: &InventorySurface,
    canonical: &InventorySurface,
    adoption: &TransitionAdoption,
) -> Result<PlanDisposition, ProductMigrationError> {
    let context_adoption = adoption.disposition == "context";
    if adoption.schema_version != ADOPTION_SCHEMA
        || source.file_kind != SurfaceFileKind::Regular
        || source.link_count != 1
        || !matches!(
            (canonical.file_kind, canonical.link_count),
            (SurfaceFileKind::Regular, 1) | (SurfaceFileKind::Semantic, 0)
        )
        || !(canonical.status == SurfaceStatus::Active
            || (context_adoption && canonical.status == SurfaceStatus::Definition))
        || adoption.source_digest_sha256 != source.digest_sha256
        || adoption.canonical_target_digest_sha256 != canonical.digest_sha256
        || !matches!(
            adoption.behavior_execution_kind.as_str(),
            BEHAVIOR_EXECUTION_KIND
                | "migration-family-agent-context-v1"
                | "migration-family-skill-wrapper-v1"
        )
        || !valid_sha256(&adoption.behavior_execution_sha256)
        || !valid_sha256(&adoption.rollback_execution_sha256)
        || !adoption.preserve_physical_bytes
        || adoption.false_pass_control_sha256.len() != REQUIRED_FALSE_PASS_CONTROLS.len()
        || REQUIRED_FALSE_PASS_CONTROLS.iter().any(|control| {
            adoption
                .false_pass_control_sha256
                .get(*control)
                .is_none_or(|value| !valid_sha256(value))
        })
    {
        return Err(ProductMigrationError::new(
            "migration-product-adopted-transition-invalid",
        ));
    }
    validate_exact_set(&adoption.exact_active_readers)?;
    validate_exact_set(&adoption.exact_active_writers)?;
    validate_exact_set(&adoption.exact_public_routes)?;
    validate_exact_set(&adoption.exact_generated_outputs)?;

    match adoption.disposition.as_str() {
        "context" => {
            if source.status != SurfaceStatus::ContextOnly
                || adoption.behavior_execution_kind != "migration-family-agent-context-v1"
                || route.transition.compatibility_behavior != "not-applicable"
                || route.transition.compatibility_boundary != "adopted"
                || route.transition.replacement_state != "candidate-required"
                || route.transition.active_reader_writer_state != "none-verified"
                || route.transition.observed_authority_state != "context-only"
                || route.transition.equivalence_proof != "not-applicable"
                || route.transition.physical_cleanup_state != "preserve"
                || adoption.post_status != "context_only"
                || !adoption.exact_active_readers.is_empty()
                || !adoption.exact_active_writers.is_empty()
                || !adoption.exact_public_routes.is_empty()
                || !adoption.exact_generated_outputs.is_empty()
                || adoption.compatibility_prerequisites.is_some()
            {
                return Err(ProductMigrationError::new(
                    "migration-product-context-adoption-refused",
                ));
            }
            Ok(PlanDisposition::AdoptContext)
        }
        "compatibility" => {
            let family_wrapper =
                adoption.behavior_execution_kind == "migration-family-skill-wrapper-v1";
            if source.status != SurfaceStatus::Active
                || route.transition.compatibility_behavior != "exact-route-only"
                || route.transition.compatibility_boundary != "explicit-only"
                || if family_wrapper {
                    route.transition.replacement_state != "candidate-required"
                        || route.transition.active_reader_writer_state != "active"
                        || route.transition.equivalence_proof != "missing"
                } else {
                    route.transition.replacement_state != "verified"
                        || route.transition.active_reader_writer_state != "none"
                        || route.transition.equivalence_proof != "executed-behavior-v1"
                }
                || route.transition.observed_authority_state != "compatibility-route-retained"
                || route.transition.physical_cleanup_state != "preserve"
                || adoption.post_status != "context_only"
                || !adoption.exact_active_readers.is_empty()
                || !adoption.exact_active_writers.is_empty()
                || adoption.exact_public_routes != [route.route_id.clone()]
                || !adoption.exact_generated_outputs.is_empty()
                || adoption
                    .compatibility_prerequisites
                    .as_ref()
                    .is_none_or(|prerequisites| {
                        !prerequisites.validate(&route.route_id, &route.canonical_target)
                    })
            {
                return Err(ProductMigrationError::new(
                    "migration-product-compatibility-adoption-refused",
                ));
            }
            Ok(PlanDisposition::AdoptCompatibility)
        }
        "retirement" => {
            if !matches!(
                source.status,
                SurfaceStatus::Active | SurfaceStatus::Candidate
            ) || route.transition.compatibility_behavior != "removed"
                || route.transition.compatibility_boundary != "closed"
                || route.transition.replacement_state != "verified"
                || route.transition.active_reader_writer_state != "none"
                || route.transition.observed_authority_state != "retired"
                || route.transition.equivalence_proof != "executed-behavior-v1"
                || route.transition.physical_cleanup_state != "preserve"
                || adoption.post_status != "retired"
                || !adoption.exact_active_readers.is_empty()
                || !adoption.exact_active_writers.is_empty()
                || !adoption.exact_public_routes.is_empty()
                || !adoption.exact_generated_outputs.is_empty()
                || adoption.compatibility_prerequisites.is_some()
            {
                return Err(ProductMigrationError::new(
                    "migration-product-retirement-adoption-refused",
                ));
            }
            Ok(PlanDisposition::RetireAuthority)
        }
        _ => Err(ProductMigrationError::new(
            "migration-product-adopted-disposition-unknown",
        )),
    }
}

fn parse_post_status(value: &str) -> Result<SurfaceStatus, ProductMigrationError> {
    match value {
        "context_only" => Ok(SurfaceStatus::ContextOnly),
        "retired" => Ok(SurfaceStatus::Retired),
        _ => Err(ProductMigrationError::new(
            "migration-product-post-status-refused",
        )),
    }
}

fn validate_exact_set(values: &[String]) -> Result<(), ProductMigrationError> {
    if values.len() > 4_096
        || values.windows(2).any(|pair| pair[0] >= pair[1])
        || values.iter().any(|value| !safe_reference(value))
    {
        return Err(ProductMigrationError::new(
            "migration-product-adopted-set-invalid",
        ));
    }
    Ok(())
}

fn reject_active_duplicate_authority(
    surfaces: &[InventorySurface],
) -> Result<(), ProductMigrationError> {
    let mut readers = BTreeMap::<&str, &str>::new();
    let mut writers = BTreeMap::<&str, &str>::new();
    let mut routes = BTreeMap::<&str, &str>::new();
    let mut generated = BTreeMap::<&str, &str>::new();
    for surface in surfaces.iter().filter(|surface| {
        surface.status == SurfaceStatus::Active
            && surface.file_kind == SurfaceFileKind::Regular
            && surface.link_count == 1
    }) {
        for (values, seen) in [
            (&surface.active_readers, &mut readers),
            (&surface.active_writers, &mut writers),
            (&surface.public_routes, &mut routes),
            (&surface.generated_outputs, &mut generated),
        ] {
            for value in values {
                if seen
                    .insert(value.as_str(), surface.stable_id.as_str())
                    .is_some()
                {
                    return Err(ProductMigrationError::new(
                        "migration-product-active-duplicate-authority",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn reject_terminal_duplicate_authority(
    surfaces: &[InventorySurface],
    effects: &[PlannedMigrationEffect],
) -> Result<(), ProductMigrationError> {
    let postconditions = effects
        .iter()
        .map(|effect| (effect.after().stable_id(), effect.after()))
        .collect::<BTreeMap<_, _>>();
    let mut readers = BTreeMap::<String, String>::new();
    let mut writers = BTreeMap::<String, String>::new();
    let mut routes = BTreeMap::<String, String>::new();
    let mut generated = BTreeMap::<String, String>::new();
    for surface in surfaces {
        let observed;
        let authority = if let Some(after) = postconditions.get(surface.stable_id.as_str()) {
            *after
        } else {
            observed = AuthoritySnapshot::from_surface(surface);
            &observed
        };
        for (values, seen) in [
            (authority.active_readers(), &mut readers),
            (authority.active_writers(), &mut writers),
            (authority.public_routes(), &mut routes),
            (authority.generated_outputs(), &mut generated),
        ] {
            for value in values {
                if seen
                    .insert(value.clone(), authority.stable_id().to_owned())
                    .is_some()
                {
                    return Err(ProductMigrationError::new(
                        "migration-product-terminal-duplicate-authority",
                    ));
                }
            }
        }
    }
    Ok(())
}
