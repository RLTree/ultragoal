mod contracts;
mod indexes;
mod templates;

use crate::audit::source_governance::GovernedSource;
use crate::generated_authority::RepositoryPath;
use std::collections::BTreeMap;

pub(super) struct ProjectionRequest<'a> {
    pub(super) output: &'a str,
    pub(super) canonical_sources: &'a [RepositoryPath],
    pub(super) sources: &'a BTreeMap<&'a str, &'a GovernedSource>,
}

pub(super) fn render(request: ProjectionRequest<'_>) -> Result<Vec<u8>, &'static str> {
    if indexes::owns(request.output) {
        indexes::render(request)
    } else {
        templates::render(request)
    }
}
