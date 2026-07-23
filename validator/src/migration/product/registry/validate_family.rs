use crate::inventory::compatibility::{
    CompiledAgentRoute, CompiledSkillRoute, agent_routes, skill_routes,
};
use crate::inventory::compatibility::{CompiledAgentTargetState, READER_PROOF_SHA256};

const FAMILY_SCHEMA: &str = "MigrationAdoptionFamily-v1";
const FAMILY_ID: &str = "n14-agent-skill-adoption";
const AGENT_BEHAVIOR_KIND: &str = "migration-family-agent-context-v1";
const SKILL_BEHAVIOR_KIND: &str = "migration-family-skill-wrapper-v1";
const REQUIRED_CONTROLS: [&str; 5] = [
    "proof-artifact",
    "receipt-production",
    "score-only",
    "test-manipulation",
    "verbosity",
];

fn validate_adoption_family(
    family: &MigrationAdoptionFamily,
    registry: &AuthorityRoutingRegistry,
    surfaces: &BTreeMap<&str, &InventorySurface>,
) -> Result<(), ProductMigrationError> {
    if family.schema_version != FAMILY_SCHEMA
        || family.family_id != FAMILY_ID
        || !family.preserve_physical_bytes
        || !valid_route_set(
            &family.agent_route_ids,
            agent_routes().map(|route| route.route_id),
        )
        || !valid_route_set(
            &family.skill_route_ids,
            skill_routes().map(|route| route.route_id),
        )
        || !valid_family_hashes(family)
        || !validate_compatibility(&family.compatibility)
    {
        return Err(ProductMigrationError::new(
            "migration-product-adoption-family-invalid",
        ));
    }

    let route_ids = family
        .agent_route_ids
        .iter()
        .chain(&family.skill_route_ids)
        .collect::<BTreeSet<_>>();
    if route_ids.len() != 28
        || registry.routes.len() != 28
        || registry
            .routes
            .iter()
            .any(|route| !route_ids.contains(&route.route_id))
    {
        return Err(ProductMigrationError::new(
            "migration-product-adoption-family-route-set-invalid",
        ));
    }
    for route in &registry.routes {
        if family
            .agent_route_ids
            .iter()
            .any(|id| id == &route.route_id)
        {
            validate_agent_route(route, surfaces, family)?;
        } else if family
            .skill_route_ids
            .iter()
            .any(|id| id == &route.route_id)
        {
            validate_skill_route(route, surfaces, family)?;
        } else {
            return Err(ProductMigrationError::new(
                "migration-product-adoption-family-route-set-invalid",
            ));
        }
    }
    Ok(())
}

fn valid_route_set<'a>(actual: &[String], expected: impl Iterator<Item = &'a str>) -> bool {
    let expected = expected.collect::<BTreeSet<_>>();
    actual.len() == expected.len()
        && actual.iter().map(String::as_str).collect::<BTreeSet<_>>() == expected
}

fn valid_family_hashes(family: &MigrationAdoptionFamily) -> bool {
    let agent_routes = agent_routes()
        .map(route_fragment)
        .collect::<Vec<_>>()
        .join("|");
    let skill_route_fragments = skill_routes()
        .map(skill_route_fragment)
        .collect::<Vec<_>>()
        .join("|");
    let agent_routes_sha = digest(agent_routes.as_bytes());
    let skill_routes_sha = digest(skill_route_fragments.as_bytes());
    let expected_agent_evidence = digest(
        format!(
            "n14-agent-context-evidence-v1|{}|{}|{}|{}|{}",
            prefixed(READER_PROOF_SHA256),
            digest(b"n14-agent-no-discovery-v1"),
            digest(b"n14-agent-no-package-v1"),
            digest(b"n14-agent-no-public-route-v1"),
            agent_routes_sha,
        )
        .as_bytes(),
    );
    let expected_skill_wrapper =
        digest(format!("n14-skill-wrapper-family-v1|{skill_routes_sha}").as_bytes());
    let expected_family_rollback = digest(
        format!(
            "n14-family-rollback-v1|{}|{}",
            family.agent_rollback_execution_sha256, family.skill_rollback_execution_sha256
        )
        .as_bytes(),
    );
    family.agent_behavior_execution_kind == AGENT_BEHAVIOR_KIND
        && family.skill_behavior_execution_kind == SKILL_BEHAVIOR_KIND
        && family.agent_behavior_execution_sha256 == digest(b"n14-agent-context-behavior-v1")
        && family.skill_behavior_execution_sha256 == digest(b"n14-skill-wrapper-forward-v1")
        && family.agent_rollback_execution_sha256 == digest(b"n14-agent-context-rollback-v1")
        && family.skill_rollback_execution_sha256 == digest(b"n14-skill-wrapper-rollback-v1")
        && family.agent_reader_evidence_sha256 == prefixed(READER_PROOF_SHA256)
        && family.agent_no_discovery_evidence_sha256 == digest(b"n14-agent-no-discovery-v1")
        && family.agent_no_package_evidence_sha256 == digest(b"n14-agent-no-package-v1")
        && family.agent_no_public_route_evidence_sha256 == digest(b"n14-agent-no-public-route-v1")
        && family.agent_evidence_sha256 == expected_agent_evidence
        && family.skill_wrapper_family_sha256 == expected_skill_wrapper
        && family.skill_no_package_evidence_sha256 == digest(b"n14-skill-no-package-v1")
        && family.skill_no_catalog_evidence_sha256 == digest(b"n14-skill-no-catalog-v1")
        && family.skill_no_profile_evidence_sha256 == digest(b"n14-skill-no-profile-v1")
        && family.skill_implicit_gateway_target == "SKILL:harness-ultragoal"
        && skill_routes().all(|route| route.canonical_name != "agentic-engineering")
        && family.family_rollback_execution_sha256 == expected_family_rollback
        && family.false_pass_control_sha256.len() == REQUIRED_CONTROLS.len()
        && REQUIRED_CONTROLS.iter().all(|control| {
            family.false_pass_control_sha256.get(*control)
                == Some(&digest(
                    format!("n14-family-false-pass-v1|{control}").as_bytes(),
                ))
        })
}

fn validate_compatibility(value: &FamilyCompatibilityPrerequisites) -> bool {
    value.owner_id == "OWN-MAINTENANCE"
        && value.user_facing_warning
            == "Deprecated compatibility route; explicit-only and non-authoritative; warning-bearing 0.0.20 window."
        && value.usage_measurement_sha256 == digest(b"n14-skill-usage-measurement-v1")
        && value.window_start_unix_ms < value.window_end_unix_ms
        && value.window_end_unix_ms - value.window_start_unix_ms <= 90 * 24 * 60 * 60 * 1_000
        && value.observed_invocations == 0
        && value.boundary_product_version == "0.0.21"
        && value.required_consecutive_windows == 1
}

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
    if source.is_none_or(|surface| {
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
    }) || target.is_none_or(|surface| {
        surface.stable_id != spec.canonical_target
            || surface.relative_path != spec.target_path
            || surface.digest_sha256 != prefixed(spec.target_sha256)
            || surface.status
                != if matches!(spec.target_state, CompiledAgentTargetState::ActiveAgent) {
                    SurfaceStatus::Active
                } else {
                    SurfaceStatus::Definition
                }
    }) || family.agent_rollback_execution_sha256.is_empty()
    {
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

fn family_adoption(
    family: &MigrationAdoptionFamily,
    route: &RegistryRoute,
) -> Result<Option<TransitionAdoption>, ProductMigrationError> {
    let is_agent = family
        .agent_route_ids
        .iter()
        .any(|id| id == &route.route_id);
    let is_skill = family
        .skill_route_ids
        .iter()
        .any(|id| id == &route.route_id);
    if !is_agent && !is_skill {
        return Ok(None);
    }
    let (
        disposition,
        post_status,
        behavior_kind,
        behavior_sha,
        rollback_sha,
        public_routes,
        prerequisites,
    ) = if is_agent {
        (
            "context",
            "context_only",
            family.agent_behavior_execution_kind.clone(),
            family.agent_behavior_execution_sha256.clone(),
            family.family_rollback_execution_sha256.clone(),
            Vec::new(),
            None,
        )
    } else {
        let prerequisites = CompatibilityPrerequisites::from_family(
            &route.route_id,
            &route.canonical_target,
            &family.compatibility.owner_id,
            &family.compatibility.user_facing_warning,
            &family.compatibility.usage_measurement_sha256,
            family.compatibility.window_start_unix_ms,
            family.compatibility.window_end_unix_ms,
            family.compatibility.observed_invocations,
            &family.compatibility.boundary_product_version,
            family.compatibility.required_consecutive_windows,
        )?;
        (
            "compatibility",
            "context_only",
            family.skill_behavior_execution_kind.clone(),
            family.skill_behavior_execution_sha256.clone(),
            family.family_rollback_execution_sha256.clone(),
            vec![route.route_id.clone()],
            Some(prerequisites),
        )
    };
    Ok(Some(TransitionAdoption {
        schema_version: ADOPTION_SCHEMA.to_owned(),
        disposition: disposition.to_owned(),
        source_digest_sha256: String::new(),
        canonical_target_digest_sha256: String::new(),
        post_status: post_status.to_owned(),
        exact_active_readers: Vec::new(),
        exact_active_writers: Vec::new(),
        exact_public_routes: public_routes,
        exact_generated_outputs: Vec::new(),
        behavior_execution_kind: behavior_kind,
        behavior_execution_sha256: behavior_sha,
        rollback_execution_sha256: rollback_sha,
        false_pass_control_sha256: family.false_pass_control_sha256.clone(),
        compatibility_prerequisites: prerequisites,
        preserve_physical_bytes: family.preserve_physical_bytes,
    }))
}

fn route_fragment(route: &CompiledAgentRoute) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        route.route_id,
        route.stable_id(),
        route.canonical_target,
        route.target_path,
        route.target_sha256
    )
}

fn skill_route_fragment(route: &CompiledSkillRoute) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        route.route_id,
        route.stable_id(),
        route.canonical_id(),
        route.metadata_path(),
        route.legacy_sha256,
        route.metadata_sha256,
        route.canonical_target_sha256
    )
}

fn prefixed(value: &str) -> String {
    if value.starts_with("sha256:") {
        value.to_owned()
    } else {
        format!("sha256:{value}")
    }
}
