mod projection;
mod retained_context;
pub(in crate::generated_authority) mod value;

use super::contracts::{GeneratedAuthorityRegistry, GeneratedSurface, RegistryProjection};
use super::parser::{RawRegistry, RawRegistryProjection, RawSurface};
use std::collections::BTreeMap;

const SCHEMA_VERSION: &str = "GeneratedSurfaceAuthority-v3";
const CONTRACT_ID: &str = "harness-ultragoal-successor-contract-v2";
const MAX_SURFACES: usize = 256;

pub(super) fn validate(raw: RawRegistry) -> Result<GeneratedAuthorityRegistry, &'static str> {
    if raw.schema_version != SCHEMA_VERSION {
        return Err("generated_authority_schema_unsupported");
    }
    if raw.contract_id != CONTRACT_ID {
        return Err("generated_authority_contract_mismatch");
    }
    if raw.surfaces.is_empty() || raw.surfaces.len() > MAX_SURFACES {
        return Err("generated_authority_surface_count_invalid");
    }
    let registry_projection = registry_projection(raw.registry_projection)?;
    let mut surfaces = BTreeMap::new();
    for raw_surface in raw.surfaces {
        let surface = surface(raw_surface)?;
        if surfaces.insert(surface.output().clone(), surface).is_some() {
            return Err("generated_authority_output_duplicate");
        }
    }
    Ok(GeneratedAuthorityRegistry {
        registry_projection,
        surfaces,
    })
}

fn registry_projection(raw: RawRegistryProjection) -> Result<RegistryProjection, &'static str> {
    let generator = super::path::parse(raw.generator)?;
    let canonical_sources = value::paths(raw.canonical_sources)?;
    value::command(&raw.regeneration_command, generator.as_str())?;
    Ok(RegistryProjection {
        generator,
        canonical_sources,
        regeneration_command: raw.regeneration_command,
    })
}

fn surface(raw: RawSurface) -> Result<GeneratedSurface, &'static str> {
    match raw {
        RawSurface::CanonicalProjection {
            output,
            generator,
            recipe,
            inputs,
        } => projection::canonical(output, generator, recipe, inputs),
        RawSurface::RetainedContext {
            output,
            sha256,
            reason,
            replacement_targets,
            preserve,
            physical_deletion_authorized,
        } => retained_context::validate(
            output,
            sha256,
            reason,
            replacement_targets,
            preserve,
            physical_deletion_authorized,
        ),
        RawSurface::SourceProjection {
            output,
            generator,
            canonical_sources,
            regeneration_command,
            output_sha256,
        } => projection::source(
            output,
            generator,
            canonical_sources,
            regeneration_command,
            output_sha256,
        ),
        RawSurface::ToolProjection {
            output,
            tool,
            tool_version,
            canonical_sources,
            regeneration_command,
            output_sha256,
        } => projection::tool(
            output,
            tool,
            tool_version,
            canonical_sources,
            regeneration_command,
            output_sha256,
        ),
    }
}
