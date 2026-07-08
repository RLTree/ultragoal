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
    pub(crate) command_args_digest: String,
    pub(crate) environment_class: &'static str,
    pub(crate) cache_class: &'static str,
    pub(crate) claim_surface: &'static str,
    pub(crate) invalidation_reason: &'static str,
}

impl ValidationSurfaceContext {
    fn authority_material(&self) -> String {
        format!(
            "input={};audit_context={};cache_key={};validator={};law={};schema={};fixture={};args={};env={};cache_class={};claim_surface={};invalidation={}",
            self.input_digest,
            self.audit_context_digest,
            self.cache_key,
            self.validator_version,
            self.law_version,
            self.schema_version,
            self.fixture_version,
            self.command_args_digest,
            self.environment_class,
            self.cache_class,
            self.claim_surface,
            self.invalidation_reason
        )
    }
}

pub(crate) fn validation_surface_context(
    root: &Path,
    candidate_digest: &str,
    node_id: &str,
    tier: &str,
    cache_mode: &str,
) -> Option<ValidationSurfaceContext> {
    let surface = surfaces::surface_by_id(node_id)?;
    let spec = surfaces::input_spec_for(node_id)?;
    let inputs = changed_inputs::ChangedInputs::collect(root, candidate_digest, tier, cache_mode);
    let input_digest = graph::surface_input_digest(
        surface,
        candidate_digest,
        inputs.surface_digest(surface),
        &inputs.audit_context_digest,
    );
    let invalidation_reason = inputs.invalidation_reason(surface);
    let context = ValidationSurfaceContext {
        cache_key: graph::verified_local_cache_key(surface, &input_digest, tier, cache_mode),
        input_digest,
        audit_context_digest: inputs.audit_context_digest,
        validator_version: graph::validator_version(),
        law_version: spec.law_version,
        schema_version: spec.schema_version,
        fixture_version: spec.fixture_version,
        command_args_digest: crate::digest::bytes(
            format!(
                "full={};narrow={};tier={tier};cache={cache_mode}",
                surface.canonical_full_command, surface.narrow_rerun
            )
            .as_bytes(),
        ),
        environment_class: spec.environment_class,
        cache_class: spec.cache_class,
        claim_surface: spec.claim_surface,
        invalidation_reason,
    };
    std::mem::drop(context.authority_material());
    Some(context)
}

#[cfg(test)]
mod tests {
    #[test]
    fn surface_context_exposes_input_spec_authority_fields() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
            "live-loop-surface-context",
        );
        std::fs::create_dir_all(&root).expect("root");

        let context = super::validation_surface_context(
            &root,
            "sha256:candidate",
            "schema_validation",
            "hot",
            "verified-local",
        )
        .expect("surface context");

        assert_eq!(
            context.law_version,
            crate::cli::live_loop::graph::law_version()
        );
        assert_eq!(
            context.schema_version,
            crate::cli::live_loop::graph::schema_version()
        );
        assert_eq!(
            context.fixture_version,
            crate::cli::live_loop::graph::fixture_version()
        );
        assert_eq!(context.environment_class, "local");
        assert_eq!(context.cache_class, "verified_content_addressed_local");
        assert_eq!(context.claim_surface, "schema_catalog");
        assert_eq!(context.invalidation_reason, "surface_inputs_unchanged");
        assert!(context.command_args_digest.starts_with("sha256:"));
        assert!(context.validator_version.starts_with("sha256:"));
        assert!(context.input_digest.starts_with("sha256:"));
        assert!(context.audit_context_digest.starts_with("sha256:"));
        assert!(context.cache_key.starts_with("sha256:"));

        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
