use super::{catalog::SURFACE_INPUT_SPECS, path_rules};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CacheBoundary {
    ChangedInputs,
    CandidatePackage,
    AuditContext,
}

#[derive(Clone, Copy)]
pub(crate) struct SurfaceInputSpec {
    pub(crate) node_id: &'static str,
    pub(crate) cache_boundary: CacheBoundary,
    pub(crate) path_rule: fn(&str) -> bool,
    pub(crate) law_version: &'static str,
    pub(crate) schema_version: &'static str,
    pub(crate) fixture_version: &'static str,
    pub(crate) validator_authority: &'static str,
    pub(crate) environment_class: &'static str,
    pub(crate) cache_class: &'static str,
    pub(crate) claim_surface: &'static str,
    pub(crate) output_digest_expectation: &'static str,
}

impl SurfaceInputSpec {
    pub(crate) fn affects_path(self, path: &str) -> bool {
        !path_rules::is_non_product_input(path) && (self.path_rule)(path)
    }

    pub(crate) fn cache_boundary_name(self) -> &'static str {
        match self.cache_boundary {
            CacheBoundary::ChangedInputs => "changed_inputs",
            CacheBoundary::CandidatePackage => "candidate_package",
            CacheBoundary::AuditContext => "audit_context",
        }
    }

    pub(crate) fn cache_material(self, tier: &str, cache_mode: &str) -> String {
        format!(
            "node={};law={};schema={};fixture={};validator={};env={};cache_class={};tier={tier};cache_mode={cache_mode};claim_surface={};output={}",
            self.node_id,
            self.law_version,
            self.schema_version,
            self.fixture_version,
            self.validator_authority,
            self.environment_class,
            self.cache_class,
            self.claim_surface,
            self.output_digest_expectation
        )
    }

    pub(crate) fn unaffected_reuse_condition(self) -> &'static str {
        match self.cache_boundary {
            CacheBoundary::CandidatePackage => {
                "requires_current_package_boundary_or_strict_boundary_proof"
            }
            CacheBoundary::AuditContext => {
                "requires_current_audit_context_versions_and_command_args"
            }
            CacheBoundary::ChangedInputs => {
                "verified_current_input_cache_hit_required_or_boundary_withheld"
            }
        }
    }

    pub(crate) fn no_changed_input_reason(self) -> &'static str {
        match self.cache_boundary {
            CacheBoundary::CandidatePackage => "candidate_package_boundary_unchanged",
            CacheBoundary::AuditContext => "audit_context_versions_and_args_unchanged",
            CacheBoundary::ChangedInputs => "surface_inputs_unchanged",
        }
    }
}

pub(crate) fn input_spec_for(node_id: &str) -> Option<SurfaceInputSpec> {
    SURFACE_INPUT_SPECS
        .iter()
        .copied()
        .find(|spec| spec.node_id == node_id)
}
