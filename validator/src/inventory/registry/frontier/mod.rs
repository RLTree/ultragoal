use super::data::CONTRACT_DIR;
use crate::context::ReadSession;
use crate::inventory::fs::{PhysicalEntryDescriptor, physical_entry, read_bounded};
use crate::inventory::types::{ActiveStatus, AuthorityState, InventoryEntry, InventoryError};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const MAX_FRONTIER_BYTES: u64 = 2 * 1024 * 1024;
const SOURCE_BASE_COMMIT: &str = "0c7f3e147d372f870751548610710353d37f9d4b";
const SOURCE_BASE_TREE: &str = "d0bbe290b2411ba89de4e90ccedeb013fbdfc1a1";

pub(super) struct Frontier {
    pub(super) active_tools: BTreeSet<String>,
    pub(super) entries: Vec<InventoryEntry>,
}

#[derive(Debug)]
struct SchedulerNodes {
    integrated: BTreeSet<String>,
    ready: BTreeSet<String>,
}

pub(super) fn load(reads: &ReadSession, root: &Path) -> Result<Frontier, InventoryError> {
    let registry_path = root.join("LANE_REGISTRY.json");
    let template_path = root.join("templates/LANE_REGISTRY.json");
    let registry = parse(reads, &registry_path)?;
    let graph = parse(
        reads,
        &root
            .join(CONTRACT_DIR)
            .join("IMPLEMENTATION_DEPENDENCY_GRAPH.json"),
    )?;
    let nodes = scheduler_nodes(&registry)?;
    scope_ownership::validate(&registry)?;
    lease_issuance::validate(reads, root, &registry, &nodes.ready)?;
    let active_tools = dependency_tools(&graph, &nodes)?;
    let entries = vec![
        physical_entry(
            reads,
            root,
            &registry_path,
            PhysicalEntryDescriptor {
                stable_id: "SCHEDULER-CONTEXT:root".to_owned(),
                kind: "scheduler-context",
                owner: "OWN-ULTRA-ROOT",
                authority_state: AuthorityState::Canonical,
                active_status: ActiveStatus::Active,
                generator: None,
                provenance: vec!["schemas/lane-registry.schema.json".to_owned()],
                references: Vec::new(),
            },
        )?,
        physical_entry(
            reads,
            root,
            &template_path,
            PhysicalEntryDescriptor {
                stable_id: "SCHEDULER-PROJECTION:template".to_owned(),
                kind: "scheduler-context-projection",
                owner: "OWN-ULTRA-ROOT",
                authority_state: AuthorityState::Projection,
                active_status: ActiveStatus::Definition,
                generator: None,
                provenance: vec!["LANE_REGISTRY.json".to_owned()],
                references: vec!["SCHEDULER-CONTEXT:root".to_owned()],
            },
        )?,
    ];
    Ok(Frontier {
        active_tools,
        entries,
    })
}

fn parse(reads: &ReadSession, path: &Path) -> Result<Value, InventoryError> {
    serde_json::from_slice(&read_bounded(reads, path, MAX_FRONTIER_BYTES)?)
        .map_err(|error| InventoryError::InvalidRegistry(error.to_string()))
}

fn scheduler_nodes(registry: &Value) -> Result<SchedulerNodes, InventoryError> {
    if registry
        .pointer("/pre_adoption_source/epoch")
        .and_then(Value::as_str)
        != Some("ADOPTED-CURRENT")
        || registry
            .pointer("/defaults/consumed/status")
            .and_then(Value::as_str)
            != Some("resolved_current")
    {
        return Err(InventoryError::InvalidRegistry(
            "scheduler frontier is not adopted and current".to_owned(),
        ));
    }
    let lanes = registry
        .get("lanes")
        .and_then(Value::as_array)
        .ok_or_else(|| InventoryError::InvalidRegistry("missing scheduler lanes".to_owned()))?;
    let mut states = BTreeMap::new();
    for lane in lanes {
        let id = lane.get("id").and_then(Value::as_str).ok_or_else(|| {
            InventoryError::InvalidRegistry("scheduler lane lacks an ID".to_owned())
        })?;
        let state = lane.get("state").and_then(Value::as_str).ok_or_else(|| {
            InventoryError::InvalidRegistry("scheduler lane lacks a state".to_owned())
        })?;
        if states.insert(id.to_owned(), state.to_owned()).is_some() {
            return Err(InventoryError::InvalidRegistry(
                "duplicate scheduler lane ID".to_owned(),
            ));
        }
    }
    let ready = states
        .iter()
        .filter(|(_, state)| state.as_str() == "ready")
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    let eligible = registry
        .pointer("/pre_adoption_source/eligible_scheduler_nodes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    if ready != eligible {
        return Err(InventoryError::InvalidRegistry(
            "scheduler eligibility disagrees with ready lanes".to_owned(),
        ));
    }
    let frontier = registry
        .pointer("/pre_adoption_source/frontier")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            InventoryError::InvalidRegistry("scheduler frontier is missing".to_owned())
        })?;
    if ready != expected_ready_lanes(frontier)? {
        return Err(InventoryError::InvalidRegistry(
            "scheduler frontier has unexpected ready lanes".to_owned(),
        ));
    }
    let integrated = states
        .into_iter()
        .filter(|(_, state)| state == "integrated")
        .map(|(id, _)| id)
        .collect();
    Ok(SchedulerNodes { integrated, ready })
}

fn expected_ready_lanes(frontier: &str) -> Result<BTreeSet<String>, InventoryError> {
    let lanes: &[&str] = match frontier {
        "N00_ADOPTION_BOUNDARY" => &["N01"],
        "N01_INTEGRATED" => &["N02"],
        "N03_INTEGRATED_DEBT_CHECKPOINT" => &[],
        "N04_N07_READY_SOURCE_FRONTIER" => &["N04", "N05", "N06", "N07"],
        "N05_N07_READY_N04_INTEGRATED_SOURCE_FRONTIER" => &["N05", "N06", "N07"],
        "N06_N07_READY_N04_N05_INTEGRATED_SOURCE_FRONTIER" => &["N06", "N07"],
        "N07_READY_N04_N06_INTEGRATED_SOURCE_FRONTIER" => &["N07"],
        "N08_N09_READY_N07_INTEGRATED_SOURCE_FRONTIER" => &["N08", "N09"],
        _ => {
            return Err(InventoryError::InvalidRegistry(
                "scheduler frontier is unknown".to_owned(),
            ));
        }
    };
    Ok(lanes.iter().map(|lane| (*lane).to_owned()).collect())
}

fn dependency_tools(
    graph: &Value,
    nodes: &SchedulerNodes,
) -> Result<BTreeSet<String>, InventoryError> {
    let rows = graph
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            InventoryError::InvalidRegistry("dependency graph lacks nodes".to_owned())
        })?;
    let mut tools = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for row in rows {
        let node = row.get("node_id").and_then(Value::as_str).ok_or_else(|| {
            InventoryError::InvalidRegistry("dependency node lacks an ID".to_owned())
        })?;
        let lane = node.get(..3).ok_or_else(|| {
            InventoryError::InvalidRegistry("dependency node ID is malformed".to_owned())
        })?;
        if !nodes.integrated.contains(lane) && !nodes.ready.contains(lane) {
            continue;
        }
        for dependency in row
            .get("depends_on")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let dependency_lane = dependency.get(..3).unwrap_or_default();
            if !nodes.integrated.contains(dependency_lane) {
                return Err(InventoryError::InvalidRegistry(
                    if nodes.ready.contains(lane) {
                        "scheduler ready frontier is not dependency closed".to_owned()
                    } else {
                        "scheduler frontier is not dependency closed".to_owned()
                    },
                ));
            }
        }
        seen.insert(lane.to_owned());
        if nodes.integrated.contains(lane) {
            tools.extend(
                row.get("required_tools")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned),
            );
        }
    }
    if !nodes.integrated.is_subset(&seen) || !nodes.ready.is_subset(&seen) {
        return Err(InventoryError::InvalidRegistry(
            "scheduler frontier names an unknown dependency node".to_owned(),
        ));
    }
    Ok(tools)
}

mod lease_issuance;
mod scope_ownership;

#[cfg(test)]
mod tests;
