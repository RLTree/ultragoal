fn validate_agent_route(
    route: &RegistryRoute,
    surfaces: &BTreeMap<&str, &InventorySurface>,
    family: &MigrationAdoptionFamily,
) -> Result<(), ProductMigrationError> {
    let Some(spec) = agent_routes().find(|spec| spec.route_id == route.route_id) else {
        return Err(ProductMigrationError::new(
            "migration-product-adoption-family-agent-route-unknown",
        ));
    };
    if route.match_spec.stable_id.as_deref() != Some(spec.stable_id().as_str())
        || route.match_spec.kind.is_some()
        || route.match_spec.relative_path.is_some()
        || route.canonical_target != spec.canonical_target
        || route.transition.proof_refs
            != [
                spec.legacy_path.to_owned(),
                spec.target_path.to_owned(),
                "docs/ultragoal-successor-live/worker-results/LEASE-N02-AGENT-READERS-002.json"
                    .to_owned(),
            ]
        || !agent_transition_is_exact(&route.transition)
    {
        return Err(ProductMigrationError::new(
            "migration-product-adoption-family-agent-route-invalid",
        ));
    }
    let source = surfaces.get(spec.stable_id().as_str()).copied();
    let target = surfaces.get(spec.canonical_target).copied();
    let source_invalid = source.is_none_or(|surface| {
        surface.kind != "legacy-agent-authority"
            || surface.relative_path != spec.legacy_path
            || surface.digest_sha256 != prefixed(spec.legacy_sha256)
            || surface.status != SurfaceStatus::ContextOnly
            || surface.file_kind != crate::migration::SurfaceFileKind::Regular
            || surface.link_count != 1
            || !surface.active_readers.is_empty()
            || !surface.active_writers.is_empty()
            || !surface.public_routes.is_empty()
            || !surface.generated_outputs.is_empty()
    });
    let target_invalid = target.is_none_or(|surface| {
        surface.stable_id != spec.canonical_target
            || surface.relative_path != spec.target_path
            || surface.digest_sha256 != prefixed(spec.target_sha256)
            || surface.status
                != if matches!(spec.target_state, CompiledAgentTargetState::ActiveAgent) {
                    SurfaceStatus::Active
                } else {
                    SurfaceStatus::Definition
                }
    });
    if source_invalid || target_invalid || family.agent_rollback_execution_sha256.is_empty() {
        return Err(ProductMigrationError::new(
            "migration-product-adoption-family-agent-observation-invalid",
        ));
    }
    Ok(())
}

fn validate_skill_route(
    route: &RegistryRoute,
    surfaces: &BTreeMap<&str, &InventorySurface>,
    family: &MigrationAdoptionFamily,
) -> Result<(), ProductMigrationError> {
    let Some(spec) = skill_routes().find(|spec| spec.route_id == route.route_id) else {
        return Err(ProductMigrationError::new(
            "migration-product-adoption-family-skill-route-unknown",
        ));
    };
    if route.match_spec.stable_id.as_deref() != Some(spec.stable_id().as_str())
        || route.match_spec.kind.is_some()
        || route.match_spec.relative_path.is_some()
        || route.canonical_target != spec.canonical_id()
        || route.transition.proof_refs != [spec.skill_path(), spec.metadata_path()]
        || !skill_transition_is_exact(&route.transition)
    {
        return Err(ProductMigrationError::new(
            "migration-product-adoption-family-skill-route-invalid",
        ));
    }
    let source = surfaces.get(spec.stable_id().as_str()).copied();
    let target = surfaces.get(spec.canonical_id().as_str()).copied();
    if source.is_none_or(|surface| {
        surface.kind != "compatibility-route-retained"
            || surface.relative_path != spec.skill_path()
            || surface.digest_sha256 != prefixed(spec.legacy_sha256)
            || surface.status != SurfaceStatus::Active
            || surface.file_kind != crate::migration::SurfaceFileKind::Regular
            || surface.link_count != 1
            || !surface.active_readers.is_empty()
            || !surface.active_writers.is_empty()
            || !surface.generated_outputs.is_empty()
    }) || target.is_none_or(|surface| {
        surface.stable_id != spec.canonical_id()
            || surface.relative_path != format!("skills/{}/SKILL.md", spec.canonical_name)
            || surface.digest_sha256 != prefixed(spec.canonical_target_sha256)
            || surface.status != SurfaceStatus::Active
    }) || family.skill_rollback_execution_sha256.is_empty()
    {
        return Err(ProductMigrationError::new(
            "migration-product-adoption-family-skill-observation-invalid",
        ));
    }
    Ok(())
}
use crate::inventory::compatibility::CompiledAgentTargetState;

fn agent_transition_is_exact(transition: &RegistryTransition) -> bool {
    transition.compatibility_behavior == "not-applicable"
        && transition.compatibility_boundary == "adopted"
        && transition.replacement_state == "candidate-required"
        && transition.active_reader_writer_state == "none-verified"
        && transition.observed_authority_state == "context-only"
        && transition.equivalence_proof == "not-applicable"
        && transition.physical_cleanup_state == "preserve"
}

fn skill_transition_is_exact(transition: &RegistryTransition) -> bool {
    transition.compatibility_behavior == "exact-route-only"
        && transition.compatibility_boundary == "explicit-only"
        && transition.replacement_state == "candidate-required"
        && transition.active_reader_writer_state == "active"
        && transition.observed_authority_state == "compatibility-route-retained"
        && transition.equivalence_proof == "missing"
        && transition.physical_cleanup_state == "preserve"
}
