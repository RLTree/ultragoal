use crate::context::ReadSession;
use crate::inventory::fs::{physical_entry, read_bounded};
use crate::inventory::types::{ActiveStatus, AuthorityState, InventoryEntry, InventoryError};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

const REGISTRY_PATH: &str = "migration/generated-surface-authority.json";
const MAX_REGISTRY_BYTES: u64 = 1024 * 1024;
const MAX_SURFACES: usize = 256;
const MAX_INPUTS: usize = 256;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: String,
    contract_id: String,
    surfaces: Vec<SurfaceSpec>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SurfaceSpec {
    pub output: String,
    pub generator: String,
    pub recipe: String,
    pub inputs: Vec<String>,
}

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.starts_with("HCT-")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte == b'-')
}

fn safe_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn validate_surface(surface: &SurfaceSpec) -> Result<(), InventoryError> {
    if !safe_path(&surface.output)
        || !["generated/", "docs/generated/", "examples/generated/"]
            .iter()
            .any(|prefix| surface.output.starts_with(prefix))
        || !safe_identifier(&surface.generator)
        || surface.recipe != "input-digest-index-v1"
        || surface.inputs.is_empty()
        || surface.inputs.len() > MAX_INPUTS
    {
        return Err(invalid("generated surface authority row is invalid"));
    }
    let mut sorted = surface.inputs.clone();
    sorted.sort();
    let unique = sorted.iter().collect::<BTreeSet<_>>();
    if surface.inputs != sorted
        || unique.len() != surface.inputs.len()
        || surface.inputs.iter().any(|input| {
            !safe_path(input) || input == &surface.output || input.starts_with(".codex-worktree/")
        })
    {
        return Err(invalid(
            "generated surface inputs are noncanonical, duplicate, or cyclic",
        ));
    }
    Ok(())
}

pub(super) fn load(
    reads: &ReadSession,
    root: &Path,
    contract_id: &str,
) -> Result<(BTreeMap<String, SurfaceSpec>, InventoryEntry), InventoryError> {
    let path = root.join(REGISTRY_PATH);
    let bytes = read_bounded(reads, &path, MAX_REGISTRY_BYTES)?;
    let registry: Registry = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("generated surface authority registry is invalid JSON"))?;
    if registry.schema_version != "GeneratedSurfaceAuthority-v1" {
        return Err(invalid(
            "unsupported generated surface authority schema version",
        ));
    }
    if registry.contract_id != contract_id {
        return Err(invalid("generated surface authority contract ID mismatch"));
    }
    if registry.surfaces.len() > MAX_SURFACES {
        return Err(invalid(
            "generated surface authority exceeds the surface-count limit",
        ));
    }
    let mut surfaces = BTreeMap::new();
    for surface in registry.surfaces {
        validate_surface(&surface)?;
        if surfaces.insert(surface.output.clone(), surface).is_some() {
            return Err(invalid(
                "generated surface authority contains a duplicate output",
            ));
        }
    }
    let entry = physical_entry(
        reads,
        root,
        &path,
        "GENERATED-SURFACE-AUTHORITY".to_owned(),
        "generated-surface-authority-registry",
        "OWN-ULTRA-ROOT",
        AuthorityState::Canonical,
        ActiveStatus::Active,
        None,
        Vec::new(),
        Vec::new(),
    )?;
    Ok((surfaces, entry))
}
