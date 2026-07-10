use super::fs::{physical_entry, read_bounded};
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use serde::Deserialize;
use std::fs;
use std::path::Path;

const ROUTES_PATH: &str = "migration/authority-routes.json";
const MAX_ROUTING_BYTES: u64 = 4 * 1024 * 1024;

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
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RouteRule {
    route_id: String,
    #[serde(rename = "match")]
    matcher: RouteMatch,
    canonical_target: String,
    disposition: String,
    compatibility_behavior: String,
    retirement_state: String,
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
}

impl RoutingData {
    pub(crate) fn apply(
        &self,
        entries: &mut std::collections::BTreeMap<String, InventoryEntry>,
        findings: &mut Vec<InventoryFinding>,
    ) {
        for entry in entries
            .values_mut()
            .filter(|entry| entry.authority_state == AuthorityState::Legacy)
        {
            let matches = self
                .registry
                .routes
                .iter()
                .filter(|route| route.matcher.matches(entry))
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [] => findings.push(InventoryFinding::error(
                    "unrouted_legacy_authority",
                    Some(&entry.stable_id),
                    Some(&entry.relative_path),
                    "legacy surface has no adopted authority disposition".to_owned(),
                )),
                [route] => {
                    entry
                        .input_provenance
                        .push(format!("{ROUTES_PATH}#/routes/{}", route.route_id));
                    entry.references.push(route.canonical_target.clone());
                    entry.normalize();
                }
                _ => findings.push(InventoryFinding::error(
                    "ambiguous_authority_route",
                    Some(&entry.stable_id),
                    Some(&entry.relative_path),
                    format!("legacy surface matches {} route rules", matches.len()),
                )),
            }
        }
    }
}

fn invalid(message: impl Into<String>) -> InventoryError {
    InventoryError::InvalidRegistry(format!("{ROUTES_PATH}: {}", message.into()))
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

fn safe_matcher(matcher: &RouteMatch) -> bool {
    matcher
        .stable_id
        .as_deref()
        .is_none_or(|value| safe_token(value, 128, b"-_.:"))
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
        if !safe_canonical_target(&route.canonical_target)
            || route.disposition != "non-authoritative"
            || route.compatibility_behavior != "unverified"
            || route.retirement_state != "blocked-by-OD-009"
        {
            return Err(invalid(
                "route overstates or malforms target, disposition, compatibility, or retirement",
            ));
        }
    }
    Ok(())
}

pub(crate) fn load(
    reads: &ReadSession,
    root: &Path,
    contract_id: &str,
) -> Result<RoutingData, InventoryError> {
    let path = root.join(ROUTES_PATH);
    let metadata = fs::symlink_metadata(&path).map_err(|error| InventoryError::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(invalid("registry must be a regular non-symlink file"));
    }
    let bytes = read_bounded(reads, &path, MAX_ROUTING_BYTES)?;
    let registry =
        serde_json::from_slice::<RouteRegistry>(&bytes).map_err(|error| InventoryError::Json {
            path: path.clone(),
            message: error.to_string(),
        })?;
    validate(&registry, contract_id)?;
    let registry_entry = physical_entry(
        reads,
        root,
        &path,
        "AUTHORITY-ROUTING-REGISTRY".to_owned(),
        "migration-authority-registry",
        "OWN-ULTRA-ROOT",
        AuthorityState::Canonical,
        ActiveStatus::Active,
        None,
        vec!["docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/MIGRATION-AND-RETIREMENT.md".to_owned()],
        Vec::new(),
    )?;
    Ok(RoutingData {
        registry_entry,
        registry,
    })
}
