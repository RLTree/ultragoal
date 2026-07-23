fn planned_effect_digest(effect: &PlannedMigrationEffect) -> Result<String, ProductMigrationError> {
    let bytes = serde_json::to_vec(&(
        "MigrationPlannedEffect-v1",
        &effect.semantic_key,
        &effect.route_id,
        &effect.canonical_target_id,
        effect.disposition,
        &effect.before,
        &effect.after,
        &effect.adopted_transition_sha256,
        &effect.behavior_execution_sha256,
        &effect.rollback_execution_sha256,
        &effect.false_pass_control_sha256,
        &effect.compatibility_prerequisites,
        &effect.compatibility_prerequisites_sha256,
    ))
    .map_err(|_| ProductMigrationError::new("migration-product-effect-invalid"))?;
    if bytes.len() > MAX_MACHINE_OUTPUT_BYTES {
        return Err(ProductMigrationError::new(
            "migration-product-effect-too-large",
        ));
    }
    Ok(digest(&bytes))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProductPlanItem {
    route_id: String,
    source_id: String,
    canonical_target_id: String,
    disposition: PlanDisposition,
    exact_active_readers: Option<Vec<String>>,
    exact_active_writers: Option<Vec<String>>,
    exact_public_routes: Option<Vec<String>>,
    exact_generated_outputs: Option<Vec<String>>,
    effect_id: Option<String>,
    reason: String,
}

impl ProductPlanItem {
    pub(super) fn issue(definition: ProductPlanItemDefinition<'_>) -> Self {
        let ProductPlanItemDefinition {
            route_id,
            source,
            canonical_target_id,
            disposition,
            effect_id,
            reason,
        } = definition;
        let exact = source.file_kind == SurfaceFileKind::Regular && source.link_count == 1;
        Self {
            route_id,
            source_id: source.stable_id.clone(),
            canonical_target_id,
            disposition,
            exact_active_readers: exact.then(|| source.active_readers.clone()),
            exact_active_writers: exact.then(|| source.active_writers.clone()),
            exact_public_routes: exact.then(|| source.public_routes.clone()),
            exact_generated_outputs: exact.then(|| source.generated_outputs.clone()),
            effect_id,
            reason: reason.to_owned(),
        }
    }
}

pub(super) struct ProductPlanItemDefinition<'a> {
    pub(super) route_id: String,
    pub(super) source: &'a InventorySurface,
    pub(super) canonical_target_id: String,
    pub(super) disposition: PlanDisposition,
    pub(super) effect_id: Option<String>,
    pub(super) reason: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProductMigrationPlanProjection {
    schema_version: String,
    input_binding: MigrationInputBinding,
    contract_id: String,
    registry_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compatibility_boundary_binding: Option<CompatibilityBoundaryBinding>,
    item_count: usize,
    effect_count: usize,
    items: Vec<ProductPlanItem>,
    effects: Vec<PlannedMigrationEffect>,
    plan_sha256: String,
    projection_sha256: String,
}

impl ProductMigrationPlanProjection {
    pub(crate) fn item_count(&self) -> usize {
        self.item_count
    }

    pub(crate) fn effect_count(&self) -> usize {
        self.effect_count
    }

    pub(crate) fn pending_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.disposition == PlanDisposition::PendingMigration)
            .count()
    }

    pub(crate) fn to_canonical_json(&self) -> Result<Vec<u8>, ProductMigrationError> {
        let bytes = serde_json::to_vec(self)
            .map_err(|_| ProductMigrationError::new("migration-product-plan-output-invalid"))?;
        if bytes.len() > MAX_MACHINE_OUTPUT_BYTES {
            return Err(ProductMigrationError::new(
                "migration-product-plan-output-too-large",
            ));
        }
        Ok(bytes)
    }
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ProductMigrationPlan {
    projection: ProductMigrationPlanProjection,
    allow_unobserved_compatibility: bool,
}

impl fmt::Debug for ProductMigrationPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProductMigrationPlan")
            .field("plan_sha256", &self.projection.plan_sha256)
            .field("item_count", &self.projection.item_count)
            .field("effect_count", &self.projection.effect_count)
            .finish()
    }
}
