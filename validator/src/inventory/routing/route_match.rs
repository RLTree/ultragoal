#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RouteMatch {
    #[serde(default)]
    stable_id: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    relative_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RouteRule {
    route_id: String,
    #[serde(rename = "match")]
    matcher: RouteMatch,
    canonical_target: String,
    intended_disposition: String,
    transition: RouteTransition,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RouteRegistry {
    schema_version: String,
    contract_id: String,
    destructive_cleanup_authorized: bool,
    authority_rule: String,
    routes: Vec<RouteRule>,
}

pub(crate) struct RoutingData {
    pub(crate) registry_entry: InventoryEntry,
    registry: RouteRegistry,
    observation: ObservedMigrationRegistry,
}

impl RoutingData {
    pub(crate) fn into_registry_observation(self) -> ObservedMigrationRegistry {
        self.observation
    }
}

fn invalid(message: impl Into<String>) -> InventoryError {
    InventoryError::InvalidRegistry(format!("{MIGRATION_REGISTRY_PATH}: {}", message.into()))
}

fn validate(registry: &RouteRegistry, contract_id: &str) -> Result<(), InventoryError> {
    if registry.schema_version != "AuthorityRoutingRegistry-v1" {
        return Err(invalid("unsupported authority routing schema version"));
    }
    if registry.contract_id != contract_id {
        return Err(invalid("authority routing contract ID mismatch"));
    }
    if registry.destructive_cleanup_authorized {
        return Err(invalid("OD-009 does not authorize destructive cleanup"));
    }
    if registry.authority_rule.trim().is_empty() {
        return Err(invalid("authority_rule must be non-empty"));
    }
    if !registry.routes.is_empty() {
        return Err(invalid(
            "retired migration routes are not current authority",
        ));
    }
    Ok(())
}
