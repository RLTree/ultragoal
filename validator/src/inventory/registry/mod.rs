mod data;
mod integrity;
mod semantic;
mod sources;
mod topology;

use super::digest::json_digest;
use super::fs::{contract_source_entry, read_bounded};
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn json(reads: &ReadSession, root: &Path, name: &str) -> Result<(PathBuf, Value), InventoryError> {
    let path = root.join(CONTRACT_DIR).join(name);
    let canonical = path.canonicalize().map_err(|error| InventoryError::Io {
        path: path.clone(),
        message: error.to_string(),
    })?;
    if !canonical.starts_with(root) {
        return Err(InventoryError::PathEscape(canonical));
    }
    let bytes = read_bounded(reads, &canonical, MAX_CONTRACT_JSON_BYTES)?;
    let value = serde_json::from_slice(&bytes).map_err(|error| InventoryError::Json {
        path: canonical.clone(),
        message: error.to_string(),
    })?;
    Ok((canonical, value))
}

fn id(row: &Value, key: &str) -> Result<String, InventoryError> {
    let value = row
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| safe_identifier(value))
        .ok_or_else(|| {
            InventoryError::InvalidRegistry(format!("row has missing or noncanonical {key}"))
        })?;
    Ok(value.to_owned())
}

fn semantic_reference(value: &str) -> bool {
    [
        "PS-", "HCT-", "CL-", "REQ-", "J-", "N0", "N1", "WS-", "SRC-",
    ]
    .iter()
    .any(|prefix| value.starts_with(prefix))
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
}

fn collect_references(value: &Value, references: &mut BTreeSet<String>) {
    match value {
        Value::Array(values) => {
            for value in values {
                collect_references(value, references);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_references(value, references);
            }
        }
        Value::String(value) if semantic_reference(value) => {
            references.insert(value.clone());
        }
        _ => {}
    }
}

fn registry_entry(
    row: &Value,
    stable_id: String,
    kind: &str,
    owner: &str,
    source: &str,
) -> Result<InventoryEntry, InventoryError> {
    let mut references = BTreeSet::new();
    collect_references(row, &mut references);
    Ok(InventoryEntry {
        stable_id: stable_id.clone(),
        kind: kind.to_owned(),
        owner_role: row
            .get("owner")
            .and_then(Value::as_str)
            .filter(|value| safe_identifier(value))
            .unwrap_or(owner)
            .to_owned(),
        relative_path: format!("{CONTRACT_DIR}/{source}#/{kind}/{stable_id}"),
        digest_sha256: json_digest(row)?,
        unix_mode: None,
        authority_state: AuthorityState::Canonical,
        active_status: ActiveStatus::Definition,
        generator: Some("HCT-INVENTORY:registry-loader".to_owned()),
        input_provenance: vec![format!("{CONTRACT_DIR}/{source}")],
        references: references.into_iter().collect(),
    })
}

fn rows<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], InventoryError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| InventoryError::InvalidRegistry(format!("missing array {key}")))
}

fn validate_count(
    value: &Value,
    count_key: &str,
    actual: usize,
    label: &str,
    findings: &mut Vec<InventoryFinding>,
) {
    let declared = value
        .get(count_key)
        .and_then(Value::as_u64)
        .map(|count| count as usize);
    if declared != Some(actual) {
        findings.push(InventoryFinding::error(
            "registry_count_mismatch",
            None,
            None,
            format!("{label} count disagrees with its rows"),
        ));
    }
}

fn required_entry(id: String, kind: &str, path: String, references: Vec<String>) -> InventoryEntry {
    InventoryEntry {
        stable_id: id,
        kind: kind.to_owned(),
        owner_role: if kind == "skill" {
            "OWN-PLUGIN-PRODUCT"
        } else {
            "OWN-ULTRA-ROOT"
        }
        .to_owned(),
        relative_path: path,
        digest_sha256: String::new(),
        unix_mode: None,
        authority_state: AuthorityState::Canonical,
        active_status: ActiveStatus::Required,
        generator: None,
        input_provenance: vec![format!("{CONTRACT_DIR}/PRODUCT_SURFACE_INVENTORY.json")],
        references,
    }
}

fn definition_rows(
    entries: &mut Vec<InventoryEntry>,
    value: &Value,
    array: &str,
    id_key: &str,
    kind: &str,
    source: &str,
) -> Result<usize, InventoryError> {
    let source_rows = rows(value, array)?;
    for row in source_rows {
        entries.push(registry_entry(
            row,
            id(row, id_key)?,
            kind,
            "OWN-ULTRA-ROOT",
            source,
        )?);
    }
    Ok(source_rows.len())
}

pub(crate) fn load(
    reads: &ReadSession,
    root: &Path,
    expected_handoff_digest: &str,
) -> Result<RegistryData, InventoryError> {
    integrity::verify(reads, root, expected_handoff_digest)?;
    let specifications = [
        (
            "PRODUCT_SURFACE_INVENTORY.json",
            "surfaces",
            "surface_id",
            "product-surface-definition",
        ),
        (
            "CUSTOM_TOOL_INVENTORY.json",
            "tools",
            "tool_id",
            "custom-tool-definition",
        ),
        (
            "CLAIM_REGISTRY.json",
            "claims",
            "claim_id",
            "claim-definition",
        ),
        (
            "REQUIREMENT_TRACE.json",
            "requirements",
            "requirement_id",
            "requirement-definition",
        ),
    ];
    let mut entries = Vec::new();
    let mut counts = BTreeMap::new();
    let mut findings = Vec::new();
    let mut contract_id = None;
    let mut product = None;
    let mut required_apis: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (name, array, id_key, kind) in specifications {
        let (path, value) = json(reads, root, name)?;
        let observed_contract = value
            .get("contract_id")
            .and_then(Value::as_str)
            .filter(|value| safe_identifier(value))
            .ok_or_else(|| InventoryError::InvalidRegistry("invalid contract_id".to_owned()))?;
        if contract_id
            .as_deref()
            .is_some_and(|known| known != observed_contract)
        {
            findings.push(InventoryFinding::error(
                "contract_id_disagreement",
                None,
                Some(name),
                "contract registry IDs disagree".to_owned(),
            ));
        }
        contract_id.get_or_insert_with(|| observed_contract.to_owned());
        let source_rows = rows(&value, array)?;
        let count_key = match array {
            "surfaces" => "surface_count",
            "tools" => "tool_count",
            "claims" => "claim_count",
            _ => "requirement_count",
        };
        validate_count(&value, count_key, source_rows.len(), array, &mut findings);
        counts.insert(array.to_owned(), source_rows.len());
        entries.push(contract_source_entry(reads, root, &path, name)?);
        for row in source_rows {
            let stable_id = id(row, id_key)?;
            if array == "tools" {
                for api in row
                    .get("public_api")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    if !safe_identifier(api) {
                        return Err(InventoryError::InvalidRegistry(
                            "custom tool has a noncanonical public API identifier".to_owned(),
                        ));
                    }
                    required_apis
                        .entry(api.to_owned())
                        .or_default()
                        .insert(stable_id.clone());
                }
            }
            entries.push(registry_entry(row, stable_id, kind, "OWN-UNKNOWN", name)?);
        }
        if array == "surfaces" {
            product = Some(value);
        }
    }
    counts.insert(
        "contract_sources".to_owned(),
        sources::load(reads, root, &mut entries)?,
    );
    let product = product.expect("surface registry loaded");
    semantic::load(
        reads,
        root,
        &product,
        required_apis,
        &mut entries,
        &mut counts,
        &mut findings,
    )?;
    let legacy_skills = topology::load(&product, &mut entries, &mut counts)?;
    Ok(RegistryData {
        contract_id: contract_id.unwrap_or_default(),
        counts,
        entries,
        findings,
        legacy_skills,
    })
}
pub(crate) use data::{CONTRACT_DIR, MAX_CONTRACT_JSON_BYTES, RegistryData, safe_identifier};
