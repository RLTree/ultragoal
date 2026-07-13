use super::super::{
    digest, safe_reference, valid_identifier, valid_sha256, valid_stable_identifier,
    InventorySurface, SurfaceStatus,
};
use super::model::{
    capture_compatibility_boundary_observation, ApplyAuthorizationAuthority, AuthoritySnapshot,
    CompatibilityBoundaryObservation, CompatibilityPrerequisites, MigrationInputBinding,
    PlanDisposition, PlannedMigrationEffect, ProductInputSnapshot, ProductMigrationError,
    ProductMigrationPlan, ProductPlanItem, REQUIRED_FALSE_PASS_CONTROLS,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const REGISTRY_SCHEMA: &str = "AuthorityRoutingRegistry-v1";
const ADOPTION_SCHEMA: &str = "MigrationTransitionAdoption-v1";
const BEHAVIOR_EXECUTION_KIND: &str = "live-behavior-execution-v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthorityRoutingRegistry {
    schema_version: String,
    contract_id: String,
    destructive_cleanup_authorized: bool,
    authority_rule: String,
    routes: Vec<RegistryRoute>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryRoute {
    route_id: String,
    #[serde(rename = "match")]
    match_spec: RegistryMatch,
    canonical_target: String,
    intended_disposition: String,
    transition: RegistryTransition,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryMatch {
    stable_id: Option<String>,
    kind: Option<String>,
    relative_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RegistryTransition {
    compatibility_behavior: String,
    compatibility_boundary: String,
    replacement_state: String,
    active_reader_writer_state: String,
    observed_authority_state: String,
    equivalence_proof: String,
    physical_cleanup_state: String,
    proof_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    adopted_effect: Option<TransitionAdoption>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TransitionAdoption {
    schema_version: String,
    disposition: String,
    source_digest_sha256: String,
    canonical_target_digest_sha256: String,
    post_status: String,
    exact_active_readers: Vec<String>,
    exact_active_writers: Vec<String>,
    exact_public_routes: Vec<String>,
    exact_generated_outputs: Vec<String>,
    behavior_execution_kind: String,
    behavior_execution_sha256: String,
    rollback_execution_sha256: String,
    false_pass_control_sha256: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compatibility_prerequisites: Option<CompatibilityPrerequisites>,
    preserve_physical_bytes: bool,
}

struct ProductPlanParts {
    input_binding: MigrationInputBinding,
    contract_id: String,
    items: Vec<ProductPlanItem>,
    effects: Vec<PlannedMigrationEffect>,
}

pub(crate) fn derive_product_plan(
    input: &ProductInputSnapshot,
    boundary_authority: Option<&dyn ApplyAuthorizationAuthority>,
) -> Result<ProductMigrationPlan, ProductMigrationError> {
    let parts = derive_product_plan_parts(input)?;
    let has_compatibility = parts
        .effects
        .iter()
        .any(|effect| effect.disposition() == PlanDisposition::AdoptCompatibility);
    let observation = match (has_compatibility, boundary_authority) {
        (true, Some(authority)) => Some(capture_compatibility_boundary_observation(authority)?),
        (true, None) => {
            return Err(ProductMigrationError::new(
                "migration-product-compatibility-boundary-authority-required",
            ));
        }
        (false, _) => None,
    };
    ProductMigrationPlan::issue(
        parts.input_binding,
        parts.contract_id,
        parts.items,
        parts.effects,
        observation,
    )
}

pub(super) fn derive_product_plan_from_bound_observation(
    input: &ProductInputSnapshot,
    observation: Option<&CompatibilityBoundaryObservation>,
) -> Result<ProductMigrationPlan, ProductMigrationError> {
    let parts = derive_product_plan_parts(input)?;
    ProductMigrationPlan::issue(
        parts.input_binding,
        parts.contract_id,
        parts.items,
        parts.effects,
        observation.cloned(),
    )
}

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
                    items.push(ProductPlanItem::issue(
                        route.route_id.clone(),
                        source,
                        route.canonical_target.clone(),
                        PlanDisposition::PendingMigration,
                        None,
                        reason,
                    ));
                }
                Some(adoption) => {
                    let disposition = validate_adoption(route, source, canonical, adoption)?;
                    let after_status = parse_post_status(&adoption.post_status)?;
                    let before = AuthoritySnapshot::from_surface(source);
                    let after = AuthoritySnapshot::adopted_postcondition(
                        source,
                        after_status,
                        adoption.exact_active_readers.clone(),
                        adoption.exact_active_writers.clone(),
                        adoption.exact_public_routes.clone(),
                        adoption.exact_generated_outputs.clone(),
                    );
                    let transition_bytes = serde_json::to_vec(adoption).map_err(|_| {
                        ProductMigrationError::new("migration-product-adopted-transition-invalid")
                    })?;
                    let effect = PlannedMigrationEffect::issue(
                        semantic_key,
                        route.route_id.clone(),
                        route.canonical_target.clone(),
                        disposition,
                        before,
                        after,
                        digest(&transition_bytes),
                        adoption.behavior_execution_sha256.clone(),
                        adoption.rollback_execution_sha256.clone(),
                        adoption.false_pass_control_sha256.clone(),
                        adoption.compatibility_prerequisites.clone(),
                    )?;
                    items.push(ProductPlanItem::issue(
                        route.route_id.clone(),
                        source,
                        route.canonical_target.clone(),
                        disposition,
                        Some(effect.effect_id().to_owned()),
                        "exact_adopted_transition",
                    ));
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

pub(crate) fn validate_adopted_registry_bytes(
    bytes: &[u8],
) -> Result<String, ProductMigrationError> {
    Ok(parse_adopted_registry(bytes)?.contract_id)
}

fn parse_adopted_registry(bytes: &[u8]) -> Result<AuthorityRoutingRegistry, ProductMigrationError> {
    if bytes.is_empty() || bytes.len() > super::model::MAX_REGISTRY_BYTES {
        return Err(ProductMigrationError::new(
            "migration-product-registry-size-refused",
        ));
    }
    let registry: AuthorityRoutingRegistry = serde_json::from_slice(bytes)
        .map_err(|_| ProductMigrationError::new("migration-product-registry-invalid"))?;
    validate_registry(&registry)?;
    Ok(registry)
}

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
        || registry.routes.is_empty()
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
            "unverified" | "exact-route-only" | "removed"
        ),
        matches!(
            transition.compatibility_boundary.as_str(),
            "blocked-by-OD-008" | "explicit-only" | "closed"
        ),
        matches!(
            transition.replacement_state.as_str(),
            "unverified" | "candidate-required" | "verified"
        ),
        matches!(
            transition.active_reader_writer_state.as_str(),
            "active" | "none"
        ),
        matches!(
            transition.observed_authority_state.as_str(),
            "active" | "compatibility-route-retained" | "retired"
        ),
        matches!(
            transition.equivalence_proof.as_str(),
            "missing" | "executed-behavior-v1"
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

fn validate_adoption(
    route: &RegistryRoute,
    source: &InventorySurface,
    canonical: &InventorySurface,
    adoption: &TransitionAdoption,
) -> Result<PlanDisposition, ProductMigrationError> {
    if adoption.schema_version != ADOPTION_SCHEMA
        || canonical.status != SurfaceStatus::Active
        || adoption.source_digest_sha256 != source.digest_sha256
        || adoption.canonical_target_digest_sha256 != canonical.digest_sha256
        || adoption.behavior_execution_kind != BEHAVIOR_EXECUTION_KIND
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
        "compatibility" => {
            if source.status != SurfaceStatus::Active
                || route.transition.compatibility_behavior != "exact-route-only"
                || route.transition.compatibility_boundary != "explicit-only"
                || route.transition.replacement_state != "verified"
                || route.transition.active_reader_writer_state != "none"
                || route.transition.observed_authority_state != "compatibility-route-retained"
                || route.transition.equivalence_proof != "executed-behavior-v1"
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
    for surface in surfaces
        .iter()
        .filter(|surface| surface.status == SurfaceStatus::Active)
    {
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
