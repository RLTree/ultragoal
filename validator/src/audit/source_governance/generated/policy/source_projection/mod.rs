mod agent_standards;
mod revalidation;

use crate::audit::source_governance::GovernedSource;
use crate::generated_authority::{RepositoryPath, Sha256Digest};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const GENERATOR: &str = "scripts/project-agent-standards";
const COMMAND: &str = "scripts/project-agent-standards write";

pub(super) struct ProjectionCheckRequest<'a> {
    pub(super) output: &'a RepositoryPath,
    pub(super) generator: &'a RepositoryPath,
    pub(super) canonical_sources: &'a [RepositoryPath],
    pub(super) regeneration_command: &'a str,
    pub(super) expected_digest: &'a Sha256Digest,
    pub(super) sources: &'a BTreeMap<&'a str, &'a GovernedSource>,
}

pub(super) struct ProjectionCheck {
    pub(super) revalidation_paths: BTreeSet<String>,
}

pub(super) fn check(request: ProjectionCheckRequest<'_>) -> Result<ProjectionCheck, String> {
    let output = request.output.as_str();
    if request.generator.as_str() != GENERATOR || request.regeneration_command != COMMAND {
        return Err(format!(
            "generated_source_projection_descriptor_unsupported:{output}"
        ));
    }
    if !request.sources.contains_key(GENERATOR) {
        return Err(format!(
            "generated_source_projection_generator_missing:{output}:{GENERATOR}"
        ));
    }
    let expected = agent_standards::render(agent_standards::ProjectionRequest {
        output,
        canonical_sources: request.canonical_sources,
        sources: request.sources,
    })
    .map_err(|detail| format!("generated_source_projection_contract_invalid:{output}:{detail}"))?;
    let actual = request
        .sources
        .get(output)
        .ok_or_else(|| format!("generated_source_projection_output_missing:{output}"))?;
    if actual.bytes != expected {
        return Err(format!("generated_source_projection_output_stale:{output}"));
    }
    let actual_digest = crate::digest::bytes(&actual.bytes);
    let expected_digest = format!("sha256:{}", request.expected_digest.lowercase_hex());
    if actual_digest != expected_digest {
        return Err(format!(
            "generated_source_projection_digest_mismatch:{output}"
        ));
    }
    let mut revalidation_paths = request
        .canonical_sources
        .iter()
        .map(|source| source.as_str().to_string())
        .collect::<BTreeSet<_>>();
    revalidation_paths.insert(GENERATOR.to_string());
    revalidation_paths.insert(output.to_string());
    Ok(ProjectionCheck { revalidation_paths })
}

pub(super) fn revalidate(
    root: &Path,
    paths: &BTreeSet<String>,
    sources: &BTreeMap<&str, &GovernedSource>,
) -> Vec<String> {
    revalidation::failures(root, paths, sources)
}
