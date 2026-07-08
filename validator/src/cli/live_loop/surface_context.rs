use super::{changed_inputs, graph, surfaces};
use std::path::Path;

pub(crate) struct ValidationSurfaceContext {
    pub(crate) input_digest: String,
    pub(crate) audit_context_digest: String,
    pub(crate) cache_key: String,
    pub(crate) validator_version: String,
    pub(crate) law_version: &'static str,
    pub(crate) schema_version: &'static str,
    pub(crate) fixture_version: &'static str,
}

pub(crate) fn validation_surface_context(
    root: &Path,
    candidate_digest: &str,
    node_id: &str,
    tier: &str,
    cache_mode: &str,
) -> Option<ValidationSurfaceContext> {
    let surface = surfaces::surface_by_id(node_id)?;
    let inputs = changed_inputs::ChangedInputs::collect(root, candidate_digest, tier, cache_mode);
    let input_digest = graph::surface_input_digest(
        surface,
        candidate_digest,
        inputs.surface_digest(surface),
        &inputs.audit_context_digest,
    );
    Some(ValidationSurfaceContext {
        cache_key: graph::verified_local_cache_key(surface, &input_digest, tier, cache_mode),
        input_digest,
        audit_context_digest: inputs.audit_context_digest,
        validator_version: graph::validator_version(),
        law_version: graph::law_version(),
        schema_version: graph::schema_version(),
        fixture_version: graph::fixture_version(),
    })
}
