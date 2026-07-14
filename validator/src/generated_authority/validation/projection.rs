use super::super::contracts::GeneratedSurface;
use super::super::path;
use super::value;

pub(super) fn canonical(
    output: String,
    generator: String,
    recipe: String,
    inputs: Vec<String>,
) -> Result<GeneratedSurface, &'static str> {
    let output = path::parse(output)?;
    let inputs = value::paths(inputs)?;
    if !generator.starts_with("HCT-")
        || recipe != "input-digest-index-v1"
        || inputs.iter().any(|input| input == &output)
    {
        return Err("generated_authority_canonical_projection_invalid");
    }
    Ok(GeneratedSurface::CanonicalProjection {
        output,
        generator,
        inputs,
    })
}

pub(super) fn source(
    output: String,
    generator: String,
    sources: Vec<String>,
    regeneration_command: String,
    digest: String,
) -> Result<GeneratedSurface, &'static str> {
    let output = path::parse(output)?;
    let generator = path::parse(generator)?;
    let canonical_sources = value::paths(sources)?;
    value::command(&regeneration_command, generator.as_str())?;
    if output == generator || canonical_sources.iter().any(|source| source == &output) {
        return Err("generated_authority_source_projection_cycle");
    }
    Ok(GeneratedSurface::SourceProjection {
        output,
        generator,
        canonical_sources,
        regeneration_command,
        output_sha256: value::digest(&digest)?,
    })
}

pub(super) fn tool(
    output: String,
    tool: String,
    tool_version: String,
    sources: Vec<String>,
    regeneration_command: String,
    digest: String,
) -> Result<GeneratedSurface, &'static str> {
    if !value::token(&tool) || !value::token(&tool_version) {
        return Err("generated_authority_tool_identity_invalid");
    }
    value::command(&regeneration_command, &tool)?;
    Ok(GeneratedSurface::ToolProjection {
        output: path::parse(output)?,
        tool,
        tool_version,
        canonical_sources: value::paths(sources)?,
        regeneration_command,
        output_sha256: value::digest(&digest)?,
    })
}
