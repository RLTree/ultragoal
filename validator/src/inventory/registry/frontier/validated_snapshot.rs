use super::{dependency_tools, parse, scheduler_nodes};
use crate::context::ReadSession;
use crate::inventory::types::InventoryError;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct ValidatedRegistry {
    pub(super) registry: Value,
    pub(super) active_tools: BTreeSet<String>,
}

pub(super) fn load(reads: &ReadSession, root: &Path) -> Result<ValidatedRegistry, InventoryError> {
    let registry = parse(reads, &root.join("LANE_REGISTRY.json"))?;
    let graph = parse(
        reads,
        &root
            .join(super::super::data::CONTRACT_DIR)
            .join("IMPLEMENTATION_DEPENDENCY_GRAPH.json"),
    )?;
    let nodes = scheduler_nodes(&registry)?;
    super::scope_ownership::validate(&registry)?;
    super::lease_issuance::validate(reads, root, &registry, &nodes)?;
    let active_tools = dependency_tools(&graph, &nodes)?;
    Ok(ValidatedRegistry {
        registry,
        active_tools,
    })
}
