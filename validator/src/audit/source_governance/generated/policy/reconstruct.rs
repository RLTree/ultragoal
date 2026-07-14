use super::{shards, source_projection, tool};
use crate::audit::source_governance::GovernedSource;
use crate::generated_authority::{GeneratedAuthorityRegistry, GeneratedSurface};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const GENERATOR: &str = "scripts/project-generated-authority";
const COMMAND: &str = "scripts/project-generated-authority write";
const SHARD_PREFIX: &str = "migration/generated-surface-authority/";

pub(super) struct Reconstruction {
    pub(super) current: bool,
    pub(super) failures: Vec<String>,
    pub(super) source_projections: BTreeSet<String>,
}

pub(super) fn current_registry(
    root: &Path,
    sources: &[GovernedSource],
    registry: &GeneratedAuthorityRegistry,
) -> Reconstruction {
    current_registry_with_between(root, sources, registry, || {})
}

fn current_registry_with_between(
    root: &Path,
    sources: &[GovernedSource],
    registry: &GeneratedAuthorityRegistry,
    between: impl FnOnce(),
) -> Reconstruction {
    let source_map = sources
        .iter()
        .map(|source| (source.relative.as_str(), source))
        .collect::<BTreeMap<_, _>>();
    let mut failures = Vec::new();
    let shard_paths = validate_registry_projection(registry, sources, &source_map, &mut failures);
    shards::validate(registry, &shard_paths, &source_map, &mut failures);
    let mut projections = BTreeSet::new();
    let mut revalidation_paths = shard_paths.iter().cloned().collect::<BTreeSet<_>>();
    revalidation_paths.insert(GENERATOR.to_string());
    revalidation_paths.insert(crate::generated_authority::REGISTRY_PATH.to_string());
    let mut tool_outputs = BTreeSet::new();
    for surface in registry.surfaces.values() {
        match surface {
            GeneratedSurface::SourceProjection {
                output,
                generator,
                canonical_sources,
                regeneration_command,
                output_sha256,
                ..
            } => {
                let output_path = output;
                let output = output_path.as_str();
                match source_projection::check(source_projection::ProjectionCheckRequest {
                    output: output_path,
                    generator,
                    canonical_sources,
                    regeneration_command,
                    expected_digest: output_sha256,
                    sources: &source_map,
                }) {
                    Ok(check) => {
                        projections.insert(output.to_string());
                        revalidation_paths.extend(check.revalidation_paths);
                    }
                    Err(failure) => failures.push(failure),
                }
            }
            GeneratedSurface::ToolProjection {
                output,
                tool: tool_name,
                tool_version,
                canonical_sources,
                output_sha256,
                ..
            } => {
                let output = output.as_str();
                if !tool_projection_current(
                    output,
                    tool_name,
                    tool_version,
                    canonical_sources.iter().map(|path| path.as_str()),
                    &output_sha256.lowercase_hex(),
                    &source_map,
                ) {
                    failures.push(format!("generated_tool_projection_invalid:{output}"));
                } else {
                    projections.insert(output.to_string());
                    tool_outputs.insert(output.to_string());
                    revalidation_paths.insert(output.to_string());
                    revalidation_paths.extend(
                        canonical_sources
                            .iter()
                            .map(|source| source.as_str().to_string()),
                    );
                }
            }
            GeneratedSurface::CanonicalProjection { .. }
            | GeneratedSurface::RetainedContext { .. } => {}
        }
    }
    for required in ["Cargo.lock", "pnpm-lock.yaml"] {
        if source_map.contains_key(required) && !tool_outputs.contains(required) {
            failures.push(format!("generated_tool_projection_required:{required}"));
        }
    }
    between();
    failures.extend(source_projection::revalidate(
        root,
        &revalidation_paths,
        &source_map,
    ));
    Reconstruction {
        current: failures.is_empty(),
        failures,
        source_projections: projections,
    }
}

#[cfg(test)]
pub(super) fn current_registry_with_between_for_test(
    root: &Path,
    sources: &[GovernedSource],
    registry: &GeneratedAuthorityRegistry,
    between: impl FnOnce(),
) -> Reconstruction {
    current_registry_with_between(root, sources, registry, between)
}

fn validate_registry_projection(
    registry: &GeneratedAuthorityRegistry,
    sources: &[GovernedSource],
    source_map: &BTreeMap<&str, &GovernedSource>,
    failures: &mut Vec<String>,
) -> Vec<String> {
    let projection = &registry.registry_projection;
    let expected = exact_shard_paths(sources, failures);
    let actual = projection
        .canonical_sources
        .iter()
        .map(|path| path.as_str().to_string())
        .collect::<Vec<_>>();
    if projection.generator.as_str() != GENERATOR
        || projection.regeneration_command != COMMAND
        || actual != expected
        || !source_map.contains_key(GENERATOR)
    {
        failures.push("generated_source_registry_projection_invalid".to_string());
    }
    expected
}

fn exact_shard_paths(sources: &[GovernedSource], failures: &mut Vec<String>) -> Vec<String> {
    let mut paths = Vec::new();
    for source in sources {
        let Some(rest) = source.relative.strip_prefix(SHARD_PREFIX) else {
            continue;
        };
        if rest.ends_with(".json") && !rest.contains('/') {
            paths.push(source.relative.clone());
        } else {
            failures.push(format!(
                "generated_source_shard_path_invalid:{}",
                source.relative
            ));
        }
    }
    paths.sort();
    if paths.is_empty() {
        failures.push("generated_source_shards_missing".to_string());
    }
    paths
}

fn tool_projection_current<'a>(
    output: &str,
    tool_name: &str,
    version: &str,
    mut canonical_sources: impl Iterator<Item = &'a str>,
    expected_digest: &str,
    sources: &BTreeMap<&str, &GovernedSource>,
) -> bool {
    canonical_sources.all(|source| sources.contains_key(source))
        && output_digest_matches(output, expected_digest, sources)
        && tool::pinned(sources, tool_name, version)
}

fn output_digest_matches(
    output: &str,
    expected: &str,
    sources: &BTreeMap<&str, &GovernedSource>,
) -> bool {
    sources.get(output).is_some_and(|source| {
        crate::digest::bytes(&source.bytes).strip_prefix("sha256:") == Some(expected)
    })
}
