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

impl RouteMatch {
    fn matches(&self, entry: &InventoryEntry) -> bool {
        self.stable_id
            .as_deref()
            .is_none_or(|value| value == entry.stable_id)
            && self.kind.as_deref().is_none_or(|value| value == entry.kind)
            && self
                .relative_path
                .as_deref()
                .is_none_or(|value| value == entry.relative_path)
    }

    fn is_empty(&self) -> bool {
        self.stable_id.is_none() && self.kind.is_none() && self.relative_path.is_none()
    }

    fn exact_stable_id(&self) -> Option<&str> {
        (self.kind.is_none() && self.relative_path.is_none())
            .then_some(self.stable_id.as_deref())
            .flatten()
    }
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
    reader_proof_is_current: bool,
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

fn safe_token(value: &str, maximum: usize, allowed: &[u8]) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || allowed.contains(&byte))
}

fn safe_route_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn safe_canonical_target(value: &str) -> bool {
    [
        "PS-",
        "HCT-",
        "SKILL:",
        "AGENT:",
        "COMMAND:",
        "CONTRACT-REGISTRY:",
    ]
    .iter()
    .any(|prefix| value.starts_with(prefix))
        && safe_token(value, 128, b"-_.:")
}

fn safe_stable_id(value: &str) -> bool {
    safe_token(value, 128, b"-_.:/")
        && !value.starts_with('/')
        && !value.contains("..")
        && !value.contains("//")
}

fn safe_matcher(matcher: &RouteMatch) -> bool {
    matcher.stable_id.as_deref().is_none_or(safe_stable_id)
        && matcher
            .kind
            .as_deref()
            .is_none_or(|value| safe_token(value, 64, b"-_"))
        && matcher.relative_path.as_deref().is_none_or(|value| {
            value.len() <= 512
                && Path::new(value)
                    .components()
                    .all(|component| matches!(component, std::path::Component::Normal(_)))
        })
}

fn safe_proof_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-' | b':' | b'#')
        })
        && !value.starts_with('/')
        && !value.contains("..")
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
    if registry.authority_rule.trim().is_empty() || registry.routes.is_empty() {
        return Err(invalid("authority_rule and routes must be non-empty"));
    }
    let mut ids = std::collections::BTreeSet::new();
    for route in &registry.routes {
        if !safe_route_id(&route.route_id) || !ids.insert(&route.route_id) {
            return Err(invalid("route_id must be bounded, canonical, and unique"));
        }
        if route.matcher.is_empty() || !safe_matcher(&route.matcher) {
            return Err(invalid("route has an empty or noncanonical matcher"));
        }
        if crate::inventory::behavioral_role::legacy_route_targets_active_role(
            route.matcher.stable_id.as_deref(),
            route.matcher.relative_path.as_deref(),
        ) {
            return Err(invalid(
                "route attempts to demote a canonical behavioral role",
            ));
        }
        if !safe_canonical_target(&route.canonical_target)
            || route.intended_disposition != "non-authoritative"
        {
            return Err(invalid("route malforms its target or intended disposition"));
        }
        let exact_matcher = route.matcher.exact_stable_id().is_some();
        let safe_proof_refs = route
            .transition
            .proof_refs
            .iter()
            .all(|reference| safe_proof_ref(reference));
        let compatibility_witness = registry_route_is_compiled(
            &route.route_id,
            route.matcher.exact_stable_id(),
            &route.canonical_target,
            &route.transition.proof_refs,
        );
        let agent_retirement_witness = agent_registry_route_is_compiled(
            &route.route_id,
            route.matcher.exact_stable_id(),
            &route.canonical_target,
            &route.transition.proof_refs,
        ) && route.transition.verifies_agent_context_transition();
        route
            .transition
            .validate(
                registry.destructive_cleanup_authorized,
                exact_matcher,
                compatibility_witness,
                agent_retirement_witness,
                safe_proof_refs,
            )
            .map_err(invalid)?;
    }
    Ok(())
}

fn has_agent_context_routes(registry: &RouteRegistry) -> bool {
    registry.routes.iter().any(|route| {
        agent_registry_route_is_compiled(
            &route.route_id,
            route.matcher.exact_stable_id(),
            &route.canonical_target,
            &route.transition.proof_refs,
        ) && route.transition.verifies_agent_context_transition()
    })
}
