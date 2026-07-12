use crate::context::ReadSession;
use crate::inventory::fs::{physical_entry, read_bounded};
use crate::inventory::types::{ActiveStatus, AuthorityState, InventoryEntry, InventoryError};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

pub(super) const REGISTRY_PATH: &str = "migration/generated-surface-authority.json";
const MAX_REGISTRY_BYTES: u64 = 1024 * 1024;
const MAX_SURFACES: usize = 256;
const MAX_INPUTS: usize = 256;
const MAX_REPLACEMENTS: usize = 256;
const MAX_REASON_BYTES: usize = 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: String,
    contract_id: String,
    surfaces: Vec<SurfaceSpec>,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum SurfaceSpec {
    CanonicalProjection {
        output: String,
        generator: String,
        recipe: String,
        inputs: Vec<String>,
    },
    RetainedContext {
        output: String,
        sha256: String,
        reason: String,
        replacement_targets: Vec<String>,
        preserve: bool,
        physical_deletion_authorized: bool,
    },
}

impl SurfaceSpec {
    pub(super) fn output(&self) -> &str {
        match self {
            Self::CanonicalProjection { output, .. } | Self::RetainedContext { output, .. } => {
                output
            }
        }
    }
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

fn upper_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
}

fn lower_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn registry_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn safe_reference(value: &str) -> bool {
    value.strip_prefix("HCT-").is_some_and(upper_token)
        || value.strip_prefix("PS-").is_some_and(upper_token)
        || value.strip_prefix("SKILL:").is_some_and(lower_token)
        || value.strip_prefix("AGENT:").is_some_and(lower_token)
        || value.strip_prefix("COMMAND:").is_some_and(lower_token)
        || value
            .strip_prefix("CONTRACT-REGISTRY:")
            .is_some_and(registry_token)
}

fn lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
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
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn validate_surface(surface: &SurfaceSpec) -> Result<(), InventoryError> {
    if !safe_path(surface.output())
        || !["generated/", "docs/generated/", "examples/generated/"]
            .iter()
            .any(|prefix| surface.output().starts_with(prefix))
    {
        return Err(invalid("generated surface authority row is invalid"));
    }
    match surface {
        SurfaceSpec::CanonicalProjection {
            output,
            generator,
            recipe,
            inputs,
        } => {
            if !safe_identifier(generator)
                || recipe != "input-digest-index-v1"
                || inputs.is_empty()
                || inputs.len() > MAX_INPUTS
            {
                return Err(invalid("canonical generated projection row is invalid"));
            }
            let mut sorted = inputs.clone();
            sorted.sort();
            let unique = sorted.iter().collect::<BTreeSet<_>>();
            if inputs != &sorted
                || unique.len() != inputs.len()
                || inputs.iter().any(|input| {
                    !safe_path(input) || input == output || input.starts_with(".codex-worktree/")
                })
            {
                return Err(invalid(
                    "generated surface inputs are noncanonical, duplicate, or cyclic",
                ));
            }
        }
        SurfaceSpec::RetainedContext {
            sha256,
            reason,
            replacement_targets,
            preserve,
            physical_deletion_authorized,
            ..
        } => {
            if !lowercase_sha256(sha256)
                || reason.is_empty()
                || reason.len() > MAX_REASON_BYTES
                || reason.trim() != reason
                || reason.bytes().any(|byte| byte.is_ascii_control())
                || replacement_targets.is_empty()
                || replacement_targets.len() > MAX_REPLACEMENTS
                || !preserve
                || *physical_deletion_authorized
            {
                return Err(invalid("retained generated context row is invalid"));
            }
            let mut sorted = replacement_targets.clone();
            sorted.sort();
            let unique = sorted.iter().collect::<BTreeSet<_>>();
            if replacement_targets != &sorted
                || unique.len() != replacement_targets.len()
                || replacement_targets
                    .iter()
                    .any(|target| !safe_reference(target))
            {
                return Err(invalid(
                    "retained generated context replacements are noncanonical or duplicate",
                ));
            }
        }
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
    if !super::json::unique_keys(&bytes) {
        return Err(invalid(
            "generated surface authority registry contains invalid JSON or duplicate keys",
        ));
    }
    let registry: Registry = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("generated surface authority registry is invalid JSON"))?;
    if registry.schema_version != "GeneratedSurfaceAuthority-v2" {
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
        let output = surface.output().to_owned();
        if surfaces.insert(output, surface).is_some() {
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
