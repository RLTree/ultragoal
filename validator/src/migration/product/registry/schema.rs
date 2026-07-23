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
    #[serde(default)]
    adoption_family: Option<MigrationAdoptionFamily>,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MigrationAdoptionFamily {
    schema_version: String,
    family_id: String,
    agent_route_ids: Vec<String>,
    skill_route_ids: Vec<String>,
    agent_behavior_execution_kind: String,
    agent_behavior_execution_sha256: String,
    agent_rollback_execution_sha256: String,
    agent_evidence_sha256: String,
    agent_reader_evidence_sha256: String,
    agent_no_discovery_evidence_sha256: String,
    agent_no_package_evidence_sha256: String,
    agent_no_public_route_evidence_sha256: String,
    skill_behavior_execution_kind: String,
    skill_behavior_execution_sha256: String,
    skill_rollback_execution_sha256: String,
    skill_wrapper_family_sha256: String,
    skill_package_manifest_surface_id: String,
    skill_package_manifest_sha256: String,
    skill_package_schema_surface_id: String,
    skill_package_schema_sha256: String,
    skill_no_package_evidence_sha256: String,
    skill_no_catalog_evidence_id: String,
    skill_no_profile_evidence_id: String,
    skill_implicit_gateway_target: String,
    skill_implicit_gateway_evidence_id: String,
    family_rollback_execution_sha256: String,
    false_pass_control_sha256: BTreeMap<String, String>,
    compatibility: FamilyCompatibilityPrerequisites,
    preserve_physical_bytes: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FamilyCompatibilityPrerequisites {
    owner_id: String,
    user_facing_warning: String,
    usage_measurement_sha256: String,
    window_start_unix_ms: u64,
    window_end_unix_ms: u64,
    observed_invocations: u64,
    boundary_product_version: String,
    required_consecutive_windows: u16,
}

struct ProductPlanParts {
    input_binding: MigrationInputBinding,
    contract_id: String,
    items: Vec<ProductPlanItem>,
    effects: Vec<PlannedMigrationEffect>,
}

pub(crate) fn derive_read_only_product_plan(
    input: &ProductInputSnapshot,
) -> Result<ProductMigrationPlan, ProductMigrationError> {
    let parts = derive_product_plan_parts(input)?;
    ProductMigrationPlan::issue(
        parts.input_binding,
        parts.contract_id,
        parts.items,
        parts.effects,
        None,
        true,
    )
}

#[cfg(test)]
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
        false,
    )
}

#[cfg(test)]
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
        false,
    )
}
