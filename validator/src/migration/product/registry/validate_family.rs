use crate::inventory::compatibility::{
    CompiledAgentRoute, CompiledSkillRoute, agent_routes, skill_routes,
};

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
    input: &ProductInputSnapshot,
) -> Result<(), ProductMigrationError> {
    let checks = (
        family.schema_version == FAMILY_SCHEMA,
        family.family_id == FAMILY_ID,
        family.preserve_physical_bytes,
        valid_route_set(
            &family.agent_route_ids,
            agent_routes().map(|route| route.route_id),
        ),
        valid_route_set(
            &family.skill_route_ids,
            skill_routes().map(|route| route.route_id),
        ),
        valid_family_hashes(family),
        validate_skill_authority_inputs(family, surfaces, input),
        validate_compatibility(&family.compatibility),
    );
    if !checks.0
        || !checks.1
        || !checks.2
        || !checks.3
        || !checks.4
        || !checks.5
        || !checks.6
        || !checks.7
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
