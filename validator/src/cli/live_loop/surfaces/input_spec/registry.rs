use super::path_rules;

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

pub(crate) const SURFACE_INPUT_SPECS: &[SurfaceInputSpec] = &[
    spec(
        "package_digest",
        CacheBoundary::CandidatePackage,
        path_rules::is_package_owned_source,
        "package_boundary",
        "package_digest_matches_current_candidate",
    ),
    spec(
        "changed_files",
        CacheBoundary::ChangedInputs,
        path_rules::always_product_path,
        "candidate_delta",
        "changed_file_list_digest",
    ),
    spec(
        "audit_context",
        CacheBoundary::AuditContext,
        path_rules::is_audit_context_input,
        "audit_context",
        "audit_context_digest",
    ),
    spec(
        "observability_control_board",
        CacheBoundary::ChangedInputs,
        path_rules::is_observability_input,
        "command_observability_inventory",
        "observability_inventory_digest",
    ),
    spec(
        "fmt_check",
        CacheBoundary::ChangedInputs,
        path_rules::is_fmt_input,
        "rust_format",
        "rustfmt_stdout_digest",
    ),
    spec(
        "build_check",
        CacheBoundary::ChangedInputs,
        path_rules::is_rust_build_surface_input,
        "rust_build",
        "cargo_build_output_digest",
    ),
    spec(
        "live_loop_measurement_rust_tests",
        CacheBoundary::ChangedInputs,
        path_rules::is_rust_build_surface_input,
        "rust_live_loop_measurement_tests",
        "focused_test_output_digest",
    ),
    spec(
        "line_caps_check",
        CacheBoundary::ChangedInputs,
        path_rules::is_rust_source,
        "source_line_caps",
        "line_cap_result_digest",
    ),
    spec(
        "namespace_check",
        CacheBoundary::ChangedInputs,
        path_rules::is_namespace_input,
        "source_namespace",
        "namespace_result_digest",
    ),
    spec(
        "schema_validation",
        CacheBoundary::ChangedInputs,
        path_rules::is_schema_input,
        "schema_catalog",
        "schema_validation_result_digest",
    ),
    spec(
        "package_inventory",
        CacheBoundary::ChangedInputs,
        path_rules::is_package_inventory_closure_input,
        "package_inventory",
        "package_inventory_result_digest",
    ),
    spec(
        "mandatory_law_validation",
        CacheBoundary::ChangedInputs,
        path_rules::is_mandatory_law_input,
        "mandatory_law_graph",
        "mandatory_law_result_digest",
    ),
    spec(
        "source_obligations_check",
        CacheBoundary::ChangedInputs,
        path_rules::is_source_obligation_input,
        "source_obligations",
        "source_obligation_result_digest",
    ),
    spec(
        "foundational_trace_check",
        CacheBoundary::ChangedInputs,
        path_rules::is_foundational_trace_input,
        "foundational_trace",
        "foundational_trace_result_digest",
    ),
    spec(
        "coverage_prove",
        CacheBoundary::ChangedInputs,
        path_rules::is_coverage_input,
        "exact_coverage",
        "coverage_receipt_digest",
    ),
    spec(
        "coverage_full_script",
        CacheBoundary::ChangedInputs,
        path_rules::is_coverage_input,
        "exact_coverage_script",
        "coverage_full_output_digest",
    ),
    spec(
        "coverage_fast_script",
        CacheBoundary::ChangedInputs,
        path_rules::is_coverage_input,
        "coverage_scope_precheck",
        "coverage_routine_output_digest",
    ),
    spec(
        "source_audit",
        CacheBoundary::ChangedInputs,
        path_rules::is_source_audit_input,
        "source_audit",
        "source_audit_receipt_digest",
    ),
    spec(
        "red_fixture_report",
        CacheBoundary::ChangedInputs,
        path_rules::is_fixture_input,
        "red_fixture_report",
        "red_fixture_report_digest",
    ),
    spec(
        "scripts_check",
        CacheBoundary::ChangedInputs,
        path_rules::is_scripts_check_input,
        "routine_shell_delegation",
        "scripts_check_output_digest",
    ),
    spec(
        "touched_fixture_reports",
        CacheBoundary::ChangedInputs,
        path_rules::is_fixture_input,
        "affected_fixture_reports",
        "affected_fixture_report_digest",
    ),
];

const fn spec(
    node_id: &'static str,
    cache_boundary: CacheBoundary,
    path_rule: fn(&str) -> bool,
    claim_surface: &'static str,
    output_digest_expectation: &'static str,
) -> SurfaceInputSpec {
    SurfaceInputSpec {
        node_id,
        cache_boundary,
        path_rule,
        law_version: "observability-live-loop",
        schema_version: "harness-ultragoal.live-loop-node-timing.v2",
        fixture_version: "source-tree-current",
        validator_authority: "ultragoal-cli-control-plane",
        environment_class: "local",
        cache_class: "verified_content_addressed_local",
        claim_surface,
        output_digest_expectation,
    }
}

pub(crate) fn input_spec_for(node_id: &str) -> Option<SurfaceInputSpec> {
    SURFACE_INPUT_SPECS
        .iter()
        .copied()
        .find(|spec| spec.node_id == node_id)
}
