use super::record::{
    LoopValidationSurface, ROUNDTRIP_REQUIRED, SAME_CANDIDATE, context_authority_artifact_surface,
    context_read_surface, hot_loop_authority_artifact_surface, hot_loop_read_surface,
};

pub(crate) const LOOP_VALIDATION_SURFACES: &[LoopValidationSurface] = &[
    context_authority_artifact_surface(
        "package_digest",
        "package_boundary",
        "ultragoal package digest",
        "target/debug/ultragoal --root . package digest",
        "target/debug/ultragoal --root . package digest",
        SAME_CANDIDATE,
    ),
    context_read_surface(
        "changed_files",
        "candidate_delta",
        "git status --short --untracked-files=all",
        "git status --short --untracked-files=all",
        "git status --short --untracked-files=all",
        SAME_CANDIDATE,
    ),
    context_read_surface(
        "audit_context",
        "audit_context",
        "AuditContext::new",
        "ultragoal loop run --tier hot --cache-mode verified-local",
        "target/debug/ultragoal --root . loop run --tier hot --cache-mode verified-local --jobs auto",
        SAME_CANDIDATE,
    ),
    context_authority_artifact_surface(
        "observability_control_board",
        "command_observability_inventory",
        "ultragoal observe prove",
        "target/debug/ultragoal --root . observe prove",
        "target/debug/ultragoal --root . observe prove",
        ROUNDTRIP_REQUIRED,
    ),
    hot_loop_read_surface(
        "fmt_check",
        "rust_format",
        "cargo fmt --all --check",
        "cargo fmt --all --check",
        "cargo fmt --all --check",
    ),
    hot_loop_authority_artifact_surface(
        "build_check",
        "rust_build",
        "cargo build --offline --bin ultragoal --quiet",
        "cargo build --offline --bin ultragoal --quiet",
        "cargo build --offline --bin ultragoal --quiet",
    ),
    hot_loop_authority_artifact_surface(
        "focused_rust_tests",
        "rust_focused_tests",
        "cargo test --offline <affected> --lib --quiet",
        "cargo test --offline --lib --quiet",
        "cargo test --offline <affected> --lib --quiet",
    ),
    hot_loop_authority_artifact_surface(
        "line_caps_check",
        "source_line_caps",
        "ultragoal line-caps check",
        "target/debug/ultragoal --root . line-caps check --strict --jobs 8",
        "target/debug/ultragoal --root . line-caps check --strict --jobs 8",
    ),
    hot_loop_authority_artifact_surface(
        "namespace_check",
        "source_namespace",
        "ultragoal namespace check",
        "target/debug/ultragoal --root . namespace check --strict --jobs 8",
        "target/debug/ultragoal --root . namespace check --strict --jobs 8",
    ),
    hot_loop_authority_artifact_surface(
        "schema_validation",
        "schema_catalog",
        "ultragoal schema validation",
        "target/debug/ultragoal --root . schema validation --jobs 8",
        "target/debug/ultragoal --root . schema validation --jobs 8",
    ),
    hot_loop_authority_artifact_surface(
        "package_inventory",
        "package_inventory",
        "ultragoal package inventory",
        "target/debug/ultragoal --root . package inventory",
        "target/debug/ultragoal --root . package inventory",
    ),
    hot_loop_authority_artifact_surface(
        "mandatory_law_validation",
        "mandatory_law_graph",
        "ultragoal mandatory-law validation",
        "target/debug/ultragoal --root . mandatory-law validation --jobs 8",
        "target/debug/ultragoal --root . mandatory-law validation --jobs 8",
    ),
    hot_loop_authority_artifact_surface(
        "source_obligations_check",
        "source_obligations",
        "ultragoal source-obligations check",
        "target/debug/ultragoal --root . source-obligations check --strict --jobs 8",
        "target/debug/ultragoal --root . source-obligations check --strict --jobs 8",
    ),
    hot_loop_authority_artifact_surface(
        "foundational_trace_check",
        "foundational_trace",
        "ultragoal foundational-trace check",
        "target/debug/ultragoal --root . foundational-trace check --strict --jobs 8",
        "target/debug/ultragoal --root . foundational-trace check --strict --jobs 8",
    ),
    hot_loop_authority_artifact_surface(
        "coverage_prove",
        "exact_coverage",
        "ultragoal coverage prove",
        "target/debug/ultragoal --root . coverage prove --receipt validation_artifacts/coverage/coverage-receipt.json --jobs 8",
        "target/debug/ultragoal --root . coverage prove --receipt validation_artifacts/coverage/coverage-receipt.json --validate-existing --jobs 8",
    ),
    hot_loop_authority_artifact_surface(
        "coverage_full_script",
        "exact_coverage_script",
        "scripts/check-coverage-full",
        "bash scripts/check-coverage-full .",
        "bash scripts/check-coverage-full .",
    ),
    hot_loop_authority_artifact_surface(
        "coverage_fast_script",
        "coverage_scope_precheck",
        "scripts/check-coverage-fast",
        "bash scripts/check-coverage-fast .",
        "bash scripts/check-coverage-fast .",
    ),
    hot_loop_authority_artifact_surface(
        "source_audit",
        "source_audit",
        "ultragoal source audit",
        "target/debug/ultragoal --root . source audit --receipt validation_artifacts/ultragoal-audit/validator-receipt.json --red-report validation_artifacts/ultragoal-audit/red-fixture-report.json --mode strict_fixtures --jobs 8",
        "target/debug/ultragoal --root . source audit --receipt validation_artifacts/ultragoal-audit/validator-receipt.json --red-report validation_artifacts/ultragoal-audit/red-fixture-report.json --mode strict_fixtures --jobs 8",
    ),
    hot_loop_authority_artifact_surface(
        "red_fixture_report",
        "red_fixture_report",
        "ultragoal red fixture report",
        "target/debug/ultragoal --root . red fixture report --report validation_artifacts/ultragoal-audit/red-fixture-report.json",
        "target/debug/ultragoal --root . red fixture report --report validation_artifacts/ultragoal-audit/red-fixture-report.json",
    ),
    hot_loop_authority_artifact_surface(
        "scripts_check",
        "routine_shell_delegation",
        "scripts/check",
        "bash scripts/check",
        "bash scripts/check",
    ),
    hot_loop_authority_artifact_surface(
        "touched_fixture_reports",
        "affected_fixture_reports",
        "affected fixture report selection",
        "target/debug/ultragoal --root . red fixture report --report validation_artifacts/ultragoal-audit/red-fixture-report.json",
        "target/debug/ultragoal --root . red fixture report --report validation_artifacts/ultragoal-audit/red-fixture-report.json",
    ),
];

pub(crate) fn surface_by_id(id: &str) -> Option<LoopValidationSurface> {
    LOOP_VALIDATION_SURFACES
        .iter()
        .copied()
        .find(|surface| surface.id == id)
}
