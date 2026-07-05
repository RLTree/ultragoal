#[derive(Clone, Copy)]
pub(crate) struct LoopValidationSurface {
    pub(crate) id: &'static str,
    pub(crate) surface: &'static str,
    pub(crate) command: &'static str,
    pub(crate) canonical_full_command: &'static str,
    pub(crate) narrow_rerun: &'static str,
    pub(crate) telemetry_reconciliation_state: &'static str,
    pub(crate) high_frequency: bool,
}

macro_rules! live_loop_surface {
    (
        $id:literal,
        $surface:literal,
        $command:literal,
        $canonical_full_command:literal,
        $narrow_rerun:literal,
        $telemetry_reconciliation_state:literal
        $(,)?
    ) => {
        LoopValidationSurface {
            id: $id,
            surface: $surface,
            command: $command,
            canonical_full_command: $canonical_full_command,
            narrow_rerun: $narrow_rerun,
            telemetry_reconciliation_state: $telemetry_reconciliation_state,
            high_frequency: true,
        }
    };
}

macro_rules! loop_context_surface {
    (
        $id:literal,
        $surface:literal,
        $command:literal,
        $canonical_full_command:literal,
        $narrow_rerun:literal,
        $telemetry_reconciliation_state:literal
        $(,)?
    ) => {
        LoopValidationSurface {
            id: $id,
            surface: $surface,
            command: $command,
            canonical_full_command: $canonical_full_command,
            narrow_rerun: $narrow_rerun,
            telemetry_reconciliation_state: $telemetry_reconciliation_state,
            high_frequency: false,
        }
    };
}

pub(crate) const LOOP_VALIDATION_SURFACES: &[LoopValidationSurface] = &[
    loop_context_surface!(
        "package_digest",
        "package_boundary",
        "ultragoal package digest",
        "target/debug/ultragoal --root . package digest",
        "target/debug/ultragoal --root . package digest",
        "same_candidate_observed",
    ),
    loop_context_surface!(
        "changed_files",
        "candidate_delta",
        "git status --short --untracked-files=all",
        "git status --short --untracked-files=all",
        "git status --short --untracked-files=all",
        "same_candidate_observed",
    ),
    loop_context_surface!(
        "audit_context",
        "audit_context",
        "AuditContext::new",
        "ultragoal loop run --tier hot --cache-mode verified-local",
        "target/debug/ultragoal --root . loop run --tier hot --cache-mode verified-local --jobs auto",
        "same_candidate_observed",
    ),
    loop_context_surface!(
        "observability_control_board",
        "command_observability_inventory",
        "ultragoal observe prove",
        "target/debug/ultragoal --root . observe prove",
        "target/debug/ultragoal --root . observe prove",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "fmt_check",
        "rust_format",
        "cargo fmt --all --check",
        "cargo fmt --all --check",
        "cargo fmt --all --check",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "build_check",
        "rust_build",
        "cargo build --offline --bin ultragoal --quiet",
        "cargo build --offline --bin ultragoal --quiet",
        "cargo build --offline --bin ultragoal --quiet",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "focused_rust_tests",
        "rust_focused_tests",
        "cargo test --offline <affected> --lib --quiet",
        "cargo test --offline --lib --quiet",
        "cargo test --offline <affected> --lib --quiet",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "line_caps_check",
        "source_line_caps",
        "ultragoal line-caps check",
        "target/debug/ultragoal --root . line-caps check --strict --jobs 8",
        "target/debug/ultragoal --root . line-caps check --strict --jobs 8",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "namespace_check",
        "source_namespace",
        "ultragoal namespace check",
        "target/debug/ultragoal --root . namespace check --strict --jobs 8",
        "target/debug/ultragoal --root . namespace check --strict --jobs 8",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "schema_validation",
        "schema_catalog",
        "ultragoal schema validation",
        "target/debug/ultragoal --root . schema validation --strict --jobs 8",
        "target/debug/ultragoal --root . schema validation --strict --jobs 8",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "package_inventory",
        "package_inventory",
        "ultragoal package inventory",
        "target/debug/ultragoal --root . package inventory",
        "target/debug/ultragoal --root . package inventory",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "mandatory_law_validation",
        "mandatory_law_graph",
        "ultragoal law check --all",
        "target/debug/ultragoal --root . law check --all",
        "target/debug/ultragoal --root . law check --all",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "source_obligations_check",
        "source_obligations",
        "ultragoal source-obligations check",
        "target/debug/ultragoal --root . source-obligations check --strict --jobs 8",
        "target/debug/ultragoal --root . source-obligations check --strict --jobs 8",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "foundational_trace_check",
        "foundational_trace",
        "ultragoal foundational-trace check",
        "target/debug/ultragoal --root . foundational-trace check --strict --jobs 8",
        "target/debug/ultragoal --root . foundational-trace check --strict --jobs 8",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "coverage_prove",
        "exact_coverage",
        "ultragoal coverage prove",
        "target/debug/ultragoal --root . coverage prove",
        "bash scripts/check-coverage-full .",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "coverage_full_script",
        "exact_coverage_script",
        "scripts/check-coverage-full",
        "bash scripts/check-coverage-full .",
        "bash scripts/check-coverage-full .",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "coverage_fast_script",
        "coverage_scope_precheck",
        "scripts/check-coverage-fast",
        "bash scripts/check-coverage-fast .",
        "bash scripts/check-coverage-fast .",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "source_audit",
        "source_audit",
        "ultragoal source audit",
        "target/debug/ultragoal --root . source audit --receipt validation_artifacts/ultragoal-audit/validator-receipt.json --red-report validation_artifacts/ultragoal-audit/red-fixture-report.json --mode strict_fixtures --jobs 8",
        "target/debug/ultragoal --root . source audit --receipt validation_artifacts/ultragoal-audit/validator-receipt.json --red-report validation_artifacts/ultragoal-audit/red-fixture-report.json --mode strict_fixtures --jobs 8",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "red_fixture_report",
        "red_fixture_report",
        "ultragoal fixtures red",
        "target/debug/ultragoal --root . fixtures red --jobs 8",
        "target/debug/ultragoal --root . fixtures red --jobs 8",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "scripts_check",
        "routine_shell_delegation",
        "scripts/check",
        "bash scripts/check",
        "bash scripts/check",
        "requires_command_telemetry_roundtrip",
    ),
    live_loop_surface!(
        "touched_fixture_reports",
        "affected_fixture_reports",
        "affected fixture report selection",
        "target/debug/ultragoal --root . fixtures all --jobs 8",
        "target/debug/ultragoal --root . fixtures all --jobs 8",
        "requires_command_telemetry_roundtrip",
    ),
];

pub(crate) fn surface_by_id(id: &str) -> Option<LoopValidationSurface> {
    LOOP_VALIDATION_SURFACES
        .iter()
        .copied()
        .find(|surface| surface.id == id)
}
