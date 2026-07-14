use crate::audit::source_governance::GovernedSource;
use crate::generated_authority::{
    GeneratedAuthorityRegistry, GeneratedAuthorityShardParseRequest, GeneratedSurface,
    GeneratedSurfaceDefinition, parse_shard,
};
use std::collections::BTreeMap;

pub(super) fn validate(
    registry: &GeneratedAuthorityRegistry,
    paths: &[String],
    sources: &BTreeMap<&str, &GovernedSource>,
    failures: &mut Vec<String>,
) {
    let mut definitions = BTreeMap::new();
    for path in paths {
        let Some(source) = sources.get(path.as_str()) else {
            failures.push(format!("generated_source_shard_missing:{path}"));
            continue;
        };
        let response = match parse_shard(GeneratedAuthorityShardParseRequest {
            bytes: &source.bytes,
        }) {
            Ok(value) => value,
            Err(error) => {
                failures.push(format!(
                    "generated_source_shard_invalid:{path}:{}",
                    error.stable_text()
                ));
                continue;
            }
        };
        if response.canonical_bytes != source.bytes {
            failures.push(format!("generated_source_shard_noncanonical:{path}"));
        }
        for (output, definition) in response.definitions {
            if definitions
                .insert(output.as_str().to_string(), definition)
                .is_some()
            {
                failures.push(format!(
                    "generated_source_shard_duplicate_output:{}",
                    output.as_str()
                ));
            }
        }
    }
    compare_registry(registry, &definitions, failures);
}

fn compare_registry(
    registry: &GeneratedAuthorityRegistry,
    definitions: &BTreeMap<String, GeneratedSurfaceDefinition>,
    failures: &mut Vec<String>,
) {
    let surfaces = registry
        .surfaces
        .values()
        .map(|surface| (surface.output().as_str(), surface))
        .collect::<BTreeMap<_, _>>();
    for (output, definition) in definitions {
        match surfaces.get(output.as_str()) {
            Some(surface) if definition_matches(definition, surface) => {}
            Some(_) => failures.push(format!("generated_source_shard_registry_mismatch:{output}")),
            None => failures.push(format!(
                "generated_source_shard_output_unregistered:{output}"
            )),
        }
    }
    for output in surfaces.keys() {
        if !definitions.contains_key(*output) {
            failures.push(format!(
                "generated_source_registry_output_undefined:{output}"
            ));
        }
    }
}

fn definition_matches(definition: &GeneratedSurfaceDefinition, surface: &GeneratedSurface) -> bool {
    match (definition, surface) {
        (
            GeneratedSurfaceDefinition::CanonicalProjection {
                output,
                generator,
                inputs,
            },
            GeneratedSurface::CanonicalProjection {
                output: actual_output,
                generator: actual_generator,
                inputs: actual_inputs,
            },
        ) => output == actual_output && generator == actual_generator && inputs == actual_inputs,
        (
            GeneratedSurfaceDefinition::RetainedContext {
                output,
                sha256,
                reason,
                replacement_targets,
            },
            GeneratedSurface::RetainedContext {
                output: actual_output,
                sha256: actual_sha256,
                reason: actual_reason,
                replacement_targets: actual_targets,
            },
        ) => {
            output == actual_output
                && sha256 == actual_sha256
                && reason == actual_reason
                && replacement_targets == actual_targets
        }
        (
            GeneratedSurfaceDefinition::SourceProjection {
                output,
                generator,
                canonical_sources,
                regeneration_command,
            },
            GeneratedSurface::SourceProjection {
                output: actual_output,
                generator: actual_generator,
                canonical_sources: actual_sources,
                regeneration_command: actual_command,
                ..
            },
        ) => {
            output == actual_output
                && generator == actual_generator
                && canonical_sources == actual_sources
                && regeneration_command == actual_command
        }
        (
            GeneratedSurfaceDefinition::ToolProjection {
                output,
                tool,
                tool_version,
                canonical_sources,
                regeneration_command,
            },
            GeneratedSurface::ToolProjection {
                output: actual_output,
                tool: actual_tool,
                tool_version: actual_version,
                canonical_sources: actual_sources,
                regeneration_command: actual_command,
                ..
            },
        ) => {
            output == actual_output
                && tool == actual_tool
                && tool_version == actual_version
                && canonical_sources == actual_sources
                && regeneration_command == actual_command
        }
        _ => false,
    }
}
