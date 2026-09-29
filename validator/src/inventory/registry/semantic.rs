use super::{CONTRACT_DIR, definition_rows, json, required_entry, safe_identifier, validate_count};
use crate::context::ReadSession;
use crate::inventory::digest::{json_digest, sha256_hex};
use crate::inventory::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) struct SemanticRegistryLoad<'a> {
    pub reads: &'a ReadSession,
    pub root: &'a Path,
    pub product: &'a Value,
    pub required_apis: BTreeMap<String, BTreeSet<String>>,
    pub active_tools: BTreeSet<String>,
    pub entries: &'a mut Vec<InventoryEntry>,
    pub counts: &'a mut BTreeMap<String, usize>,
    pub findings: &'a mut Vec<InventoryFinding>,
}

pub(super) fn load(request: SemanticRegistryLoad<'_>) -> Result<(), InventoryError> {
    let SemanticRegistryLoad {
        reads,
        root,
        product,
        required_apis,
        active_tools,
        entries,
        counts,
        findings,
    } = request;
    let journey_count = definition_rows(
        entries,
        product,
        "journeys",
        "journey_id",
        "journey-definition",
        "PRODUCT_SURFACE_INVENTORY.json",
    )?;
    validate_count(
        product,
        "journey_count",
        journey_count,
        "journeys",
        findings,
    );
    counts.insert("journeys".to_owned(), journey_count);
    if let Some(layers) = product.get("truth_layers").and_then(Value::as_array) {
        for layer in layers.iter().filter_map(Value::as_str) {
            if !safe_identifier(layer) {
                return Err(InventoryError::InvalidRegistry(
                    "truth layer has a noncanonical identifier".to_owned(),
                ));
            }
            entries.push(InventoryEntry {
                stable_id: format!("TRUTH-LAYER:{layer}"),
                kind: "truth-layer-definition".to_owned(),
                owner_role: "OWN-PRODUCT-ARCHITECTURE".to_owned(),
                relative_path: format!(
                    "{CONTRACT_DIR}/PRODUCT_SURFACE_INVENTORY.json#/truth_layers/{layer}"
                ),
                digest_sha256: json_digest(&Value::String(layer.to_owned()))?,
                unix_mode: None,
                authority_state: AuthorityState::Canonical,
                active_status: ActiveStatus::Definition,
                generator: Some("HCT-INVENTORY:registry-loader".to_owned()),
                input_provenance: vec![format!("{CONTRACT_DIR}/PRODUCT_SURFACE_INVENTORY.json")],
                references: Vec::new(),
            });
        }
        counts.insert("truth_layers".to_owned(), layers.len());
    }
    let (_, graph) = json(reads, root, "IMPLEMENTATION_DEPENDENCY_GRAPH.json")?;
    let node_count = definition_rows(
        entries,
        &graph,
        "nodes",
        "node_id",
        "dependency-node-definition",
        "IMPLEMENTATION_DEPENDENCY_GRAPH.json",
    )?;
    let scope_count = definition_rows(
        entries,
        &graph,
        "write_scopes",
        "scope_id",
        "write-scope-definition",
        "IMPLEMENTATION_DEPENDENCY_GRAPH.json",
    )?;
    counts.insert("dependency_nodes".to_owned(), node_count);
    counts.insert("write_scopes".to_owned(), scope_count);
    let (_, research) = json(reads, root, "RESEARCH_SOURCE_REGISTRY.json")?;
    let research_count = definition_rows(
        entries,
        &research,
        "sources",
        "source_id",
        "research-source-definition",
        "RESEARCH_SOURCE_REGISTRY.json",
    )?;
    validate_count(
        &research,
        "source_count",
        research_count,
        "research sources",
        findings,
    );
    counts.insert("research_sources".to_owned(), research_count);
    let required_api_count = required_apis.len();
    let required_api_names = required_apis.keys().cloned().collect::<BTreeSet<_>>();
    let bound_apis = crate::cli::successor_public::active_api_identifiers();
    let active_api_names = required_apis
        .iter()
        .filter(|(api, tools)| {
            bound_apis.contains(api.as_str())
                && tools.iter().any(|tool| active_tools.contains(tool))
        })
        .map(|(api, _)| api.clone())
        .collect::<BTreeSet<_>>();
    for (api, tools) in required_apis {
        let mut entry = required_entry(
            format!("API:{api}"),
            "source-symbol-implementation",
            format!("@semantic/{api}"),
            tools.into_iter().collect(),
        );
        if !active_api_names.contains(&api) {
            entry.active_status = ActiveStatus::Definition;
        }
        entries.push(entry);
    }
    for api in crate::api_witness::compatible_public_apis() {
        if !required_api_names.contains(*api) {
            return Err(InventoryError::InvalidRegistry(
                "compiled API witness is absent from the adopted tool contract".to_owned(),
            ));
        }
        entries.push(InventoryEntry {
            stable_id: format!("API:{api}"),
            kind: "source-symbol-implementation".to_owned(),
            owner_role: "OWN-AUTHORITY-KERNEL".to_owned(),
            relative_path: format!("@compiled-api/{api}"),
            digest_sha256: sha256_hex(api.as_bytes()),
            unix_mode: None,
            authority_state: AuthorityState::Canonical,
            active_status: if active_api_names.contains(*api) {
                ActiveStatus::Active
            } else {
                ActiveStatus::Definition
            },
            generator: Some("HCT-INVENTORY:compiled-api-witness".to_owned()),
            input_provenance: vec![
                "validator/src/api_witness.rs".to_owned(),
                "validator/tests/public_api_witness.rs".to_owned(),
            ],
            references: Vec::new(),
        });
    }
    counts.insert("required_public_api_symbols".to_owned(), required_api_count);
    Ok(())
}
