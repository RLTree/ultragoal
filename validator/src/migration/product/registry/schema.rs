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
