use std::collections::{BTreeMap, BTreeSet};

use super::model::{CheckNode, ClaimBoundary, PathRoute};
use crate::routine_work::{RoutineError, RoutineErrorId};

const MAX_NODES: usize = 4_096;
const MAX_ROUTES: usize = 16_384;
const MAX_CLAIMS: usize = 4_096;

pub(super) fn validate_graph_size(
    nodes: usize,
    routes: usize,
    claims: usize,
) -> Result<(), RoutineError> {
    if nodes > MAX_NODES || routes > MAX_ROUTES || claims > MAX_CLAIMS {
        return Err(registry_error("impact-graph-limit-exceeded"));
    }
    Ok(())
}

pub(super) fn unique_nodes(
    nodes: Vec<CheckNode>,
) -> Result<BTreeMap<String, CheckNode>, RoutineError> {
    let mut result = BTreeMap::new();
    for node in nodes {
        if result.insert(node.node_id.clone(), node).is_some() {
            return Err(registry_error("duplicate-node-row"));
        }
    }
    if result.is_empty() {
        return Err(registry_error("empty-impact-graph"));
    }
    Ok(result)
}

pub(super) fn validate_dependencies(
    nodes: &BTreeMap<String, CheckNode>,
) -> Result<(), RoutineError> {
    for node in nodes.values() {
        if node
            .depends_on
            .iter()
            .any(|dependency| !nodes.contains_key(dependency))
        {
            return Err(RoutineError::new(
                RoutineErrorId::UnknownRegistryRow,
                "dependency-target-unknown",
                None,
            ));
        }
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for node_id in nodes.keys() {
        visit(node_id, nodes, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn visit(
    node_id: &str,
    nodes: &BTreeMap<String, CheckNode>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> Result<(), RoutineError> {
    if visited.contains(node_id) {
        return Ok(());
    }
    if !visiting.insert(node_id.to_owned()) {
        return Err(registry_error("dependency-cycle"));
    }
    for dependency in &nodes[node_id].depends_on {
        visit(dependency, nodes, visiting, visited)?;
    }
    visiting.remove(node_id);
    visited.insert(node_id.to_owned());
    Ok(())
}

pub(super) fn validate_routes(
    nodes: &BTreeMap<String, CheckNode>,
    routes: &[PathRoute],
) -> Result<(), RoutineError> {
    let mut rows = BTreeSet::new();
    let mut matchers = BTreeSet::new();
    let mut case_keys = BTreeSet::new();
    for route in routes {
        if !rows.insert(route.row_id.clone()) {
            return Err(registry_error("duplicate-route-row"));
        }
        if !matchers.insert(route.matcher.clone()) {
            return Err(ambiguous("duplicate-path-matcher"));
        }
        if !case_keys.insert(route.matcher.case_key()) {
            return Err(ambiguous("case-alias-path-matcher"));
        }
        if route.node_ids.iter().any(|node| !nodes.contains_key(node)) {
            return Err(RoutineError::new(
                RoutineErrorId::UnknownRegistryRow,
                "route-target-unknown",
                None,
            ));
        }
    }
    Ok(())
}

pub(super) fn unique_claims(
    nodes: &BTreeMap<String, CheckNode>,
    claims: Vec<ClaimBoundary>,
) -> Result<BTreeMap<String, ClaimBoundary>, RoutineError> {
    let mut result = BTreeMap::new();
    for claim in claims {
        if claim.node_ids.iter().any(|node| !nodes.contains_key(node)) {
            return Err(RoutineError::new(
                RoutineErrorId::UnknownRegistryRow,
                "claim-target-unknown",
                None,
            ));
        }
        if result.insert(claim.claim_id.clone(), claim).is_some() {
            return Err(registry_error("duplicate-claim-row"));
        }
    }
    Ok(result)
}

pub(super) fn identifier(value: String) -> Result<String, RoutineError> {
    let valid = !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'));
    if !valid {
        return Err(RoutineError::new(
            RoutineErrorId::InvalidRegistry,
            "semantic-identifier-invalid",
            Some(value.as_bytes()),
        ));
    }
    Ok(value)
}

pub(super) fn identifiers(
    values: impl IntoIterator<Item = String>,
    duplicate_cause: &'static str,
) -> Result<BTreeSet<String>, RoutineError> {
    let mut result = BTreeSet::new();
    for value in values {
        if !result.insert(identifier(value)?) {
            return Err(registry_error(duplicate_cause));
        }
    }
    Ok(result)
}

pub(super) fn registry_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRegistry, cause, None)
}

fn ambiguous(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::AmbiguousRegistry, cause, None)
}
