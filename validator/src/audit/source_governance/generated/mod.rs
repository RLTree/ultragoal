pub(super) mod authority_file;
mod policy;
mod registry;

use super::GovernedSource;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct GeneratedValidation {
    pub(super) failures: Vec<String>,
    pub(super) source_projections: BTreeSet<String>,
}

pub(super) fn validate(root: &Path, sources: &[GovernedSource]) -> GeneratedValidation {
    let mut result = match registry::load(sources) {
        Ok(loaded) => {
            let mut result = policy::validate(root, sources, &loaded.registry);
            if let Err(failure) = registry::revalidate(root, loaded.source) {
                result.failures.push(failure);
            }
            result
        }
        Err(failure) => GeneratedValidation {
            failures: vec![failure],
            source_projections: BTreeSet::new(),
        },
    };
    for source in sources {
        if generated_marker(&source.bytes) && !result.source_projections.contains(&source.relative)
        {
            result
                .failures
                .push(format!("generated_source_unregistered:{}", source.relative));
        }
    }
    result.failures.sort();
    result.failures.dedup();
    result
}

#[cfg(test)]
pub(super) fn projection_failures_with_between(
    root: &Path,
    sources: &[GovernedSource],
    between: impl FnOnce(),
) -> Vec<String> {
    match registry::load(sources) {
        Ok(loaded) => {
            let mut failures =
                policy::projection_failures_with_between(root, sources, &loaded.registry, between);
            if let Err(failure) = registry::revalidate(root, loaded.source) {
                failures.push(failure);
            }
            failures.sort();
            failures.dedup();
            failures
        }
        Err(failure) => vec![failure],
    }
}

fn generated_marker(bytes: &[u8]) -> bool {
    String::from_utf8_lossy(bytes)
        .lines()
        .take(8)
        .map(str::to_ascii_lowercase)
        .any(|line| {
            line.contains("@generated")
                || line.contains("generated file")
                || line.contains("do not edit")
        })
}
