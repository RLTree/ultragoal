mod reconstruct;
mod retained;
mod shards;
mod source_projection;
mod tool;

use super::GeneratedValidation;
use crate::audit::source_governance::GovernedSource;
use crate::generated_authority::{GeneratedAuthorityRegistry, GeneratedSurface};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) fn validate(
    root: &Path,
    sources: &[GovernedSource],
    registry: &GeneratedAuthorityRegistry,
) -> GeneratedValidation {
    let mut failures = Vec::new();
    let mut projections = BTreeSet::new();
    let result = reconstruct::current_registry(root, sources, registry);
    failures.extend(result.failures);
    if result.current {
        projections.extend(result.source_projections);
        projections.insert(crate::generated_authority::REGISTRY_PATH.to_string());
    }
    for surface in registry.surfaces.values() {
        if let GeneratedSurface::RetainedContext {
            output,
            sha256,
            reason,
            replacement_targets,
        } = surface
        {
            failures.extend(retained::failures(
                root,
                output,
                sha256,
                reason,
                replacement_targets,
            ));
        }
    }
    GeneratedValidation {
        failures,
        source_projections: projections,
    }
}

#[cfg(test)]
pub(super) fn projection_failures_with_between(
    root: &Path,
    sources: &[GovernedSource],
    registry: &GeneratedAuthorityRegistry,
    between: impl FnOnce(),
) -> Vec<String> {
    reconstruct::current_registry_with_between_for_test(root, sources, registry, between).failures
}
