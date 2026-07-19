use super::super::path;
use super::super::validation::value;
use super::contracts::GeneratedSurfaceDefinition;
use super::parser::{RawDefinition, RawShard};
use std::collections::BTreeMap;

const SCHEMA_VERSION: &str = "GeneratedSurfaceAuthorityShard-v1";
const CONTRACT_ID: &str = "harness-ultragoal-successor-contract-v2";
const MAX_DEFINITIONS: usize = 256;

pub(super) fn validate(
    raw: RawShard,
) -> Result<
    BTreeMap<super::super::contracts::RepositoryPath, GeneratedSurfaceDefinition>,
    &'static str,
> {
    if raw.schema_version != SCHEMA_VERSION {
        return Err("generated_authority_shard_schema_unsupported");
    }
    if raw.contract_id != CONTRACT_ID {
        return Err("generated_authority_shard_contract_mismatch");
    }
    if raw.surfaces.is_empty() || raw.surfaces.len() > MAX_DEFINITIONS {
        return Err("generated_authority_shard_definition_count_invalid");
    }
    let mut definitions = BTreeMap::new();
    for raw_definition in raw.surfaces {
        let definition = definition(raw_definition)?;
        if definitions
            .insert(definition.output().clone(), definition)
            .is_some()
        {
            return Err("generated_authority_shard_output_duplicate");
        }
    }
    Ok(definitions)
}

fn definition(raw: RawDefinition) -> Result<GeneratedSurfaceDefinition, &'static str> {
    match raw {
        RawDefinition::CanonicalProjection {
            output,
            generator,
            recipe,
            inputs,
        } => canonical(output, generator, recipe, inputs),
        RawDefinition::RetainedContext {
            output,
            sha256,
            reason,
            replacement_targets,
            preserve,
            physical_deletion_authorized,
        } => retained(
            output,
            sha256,
            reason,
            replacement_targets,
            preserve,
            physical_deletion_authorized,
        ),
        RawDefinition::SourceProjection {
            output,
            generator,
            canonical_sources,
            regeneration_command,
        } => source(output, generator, canonical_sources, regeneration_command),
        RawDefinition::ToolProjection {
            output,
            tool,
            tool_version,
            canonical_sources,
            regeneration_command,
        } => tool_definition(
            output,
            tool,
            tool_version,
            canonical_sources,
            regeneration_command,
        ),
    }
}

fn canonical(
    output: String,
    generator: String,
    recipe: String,
    inputs: Vec<String>,
) -> Result<GeneratedSurfaceDefinition, &'static str> {
    let output = path::parse(output)?;
    let inputs = value::paths(inputs)?;
    if !generator.starts_with("HCT-")
        || recipe != "input-digest-index-v1"
        || inputs.iter().any(|input| input == &output)
    {
        return Err("generated_authority_shard_canonical_projection_invalid");
    }
    Ok(GeneratedSurfaceDefinition::CanonicalProjection {
        output,
        generator,
        inputs,
    })
}

fn retained(
    output: String,
    digest: String,
    reason: String,
    replacement_targets: Vec<String>,
    preserve: bool,
    deletion: bool,
) -> Result<GeneratedSurfaceDefinition, &'static str> {
    if !preserve
        || deletion
        || reason.trim().is_empty()
        || reason.len() > 1024
        || !value::replacement_targets(&replacement_targets)
    {
        return Err("generated_authority_shard_retained_context_invalid");
    }
    Ok(GeneratedSurfaceDefinition::RetainedContext {
        output: path::parse(output)?,
        sha256: value::digest(&digest)?,
        reason,
        replacement_targets,
    })
}

fn source(
    output: String,
    generator: String,
    sources: Vec<String>,
    regeneration_command: String,
) -> Result<GeneratedSurfaceDefinition, &'static str> {
    let output = path::parse(output)?;
    let generator = path::parse(generator)?;
    let canonical_sources = value::paths(sources)?;
    value::command(&regeneration_command, generator.as_str())?;
    if output == generator || canonical_sources.iter().any(|source| source == &output) {
        return Err("generated_authority_shard_source_projection_cycle");
    }
    Ok(GeneratedSurfaceDefinition::SourceProjection {
        output,
        generator,
        canonical_sources,
        regeneration_command,
    })
}

fn tool_definition(
    output: String,
    tool: String,
    tool_version: String,
    sources: Vec<String>,
    regeneration_command: String,
) -> Result<GeneratedSurfaceDefinition, &'static str> {
    if !value::token(&tool) || !value::token(&tool_version) {
        return Err("generated_authority_shard_tool_identity_invalid");
    }
    value::command(&regeneration_command, &tool)?;
    Ok(GeneratedSurfaceDefinition::ToolProjection {
        output: path::parse(output)?,
        tool,
        tool_version,
        canonical_sources: value::paths(sources)?,
        regeneration_command,
    })
}
