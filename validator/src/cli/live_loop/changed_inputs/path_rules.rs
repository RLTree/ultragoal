pub(super) fn path_affects_surface(path: &str, surface_id: &str) -> bool {
    if !surface_has_explicit_input_spec(surface_id) || is_non_product_input(path) {
        return false;
    }
    match surface_id {
        "package_digest" => is_package_owned_source(path),
        "changed_files" => true,
        "audit_context" => is_audit_context_input(path),
        "observability_control_board" => is_observability_input(path),
        "fmt_check" => is_rust_source(path) || is_rust_format_config(path),
        "build_check" | "live_loop_measurement_rust_tests" => {
            is_rust_source(path) || is_rust_build_input(path)
        }
        "line_caps_check" => is_rust_source(path),
        "namespace_check" => {
            is_rust_source(path)
                || path == "docs/namespace-law-exceptions.json"
                || path == "plugin-manifest-draft.json"
        }
        "schema_validation" => is_json_surface(path) || path.starts_with("schemas/"),
        "package_inventory" => is_package_inventory_closure_input(path),
        "mandatory_law_validation" => is_law_input(path) || is_receipt_input(path),
        "source_obligations_check" => is_source_obligation_input(path),
        "foundational_trace_check" => is_foundational_trace_input(path),
        "coverage_prove" | "coverage_full_script" | "coverage_fast_script" => {
            is_coverage_input(path)
        }
        "source_audit" => {
            is_package_inventory_closure_input(path)
                || is_package_boundary_input(path)
                || is_package_owned_source(path)
                || is_law_input(path)
                || is_receipt_input(path)
                || is_fixture_input(path)
        }
        "red_fixture_report" | "touched_fixture_reports" => is_fixture_input(path),
        "scripts_check" => is_scripts_check_input(path),
        _ => false,
    }
}

pub(super) fn surface_has_explicit_input_spec(surface_id: &str) -> bool {
    matches!(
        surface_id,
        "package_digest"
            | "changed_files"
            | "audit_context"
            | "observability_control_board"
            | "fmt_check"
            | "build_check"
            | "live_loop_measurement_rust_tests"
            | "line_caps_check"
            | "namespace_check"
            | "schema_validation"
            | "package_inventory"
            | "mandatory_law_validation"
            | "source_obligations_check"
            | "foundational_trace_check"
            | "coverage_prove"
            | "coverage_full_script"
            | "coverage_fast_script"
            | "source_audit"
            | "red_fixture_report"
            | "scripts_check"
            | "touched_fixture_reports"
    )
}

fn is_non_product_input(path: &str) -> bool {
    crate::package::inventory::builder_contract_resource_path(path)
        || is_local_review_state(path)
        || path.starts_with("target/")
        || path.contains("/target/")
}

fn is_local_review_state(path: &str) -> bool {
    matches!(
        path,
        "state/codex-review-artifacts" | "state/codex-review-receipts.d"
    ) || path.starts_with("state/codex-review-artifacts/")
        || path.starts_with("state/codex-review-receipts.d/")
}

fn is_package_boundary_input(path: &str) -> bool {
    is_package_root_resource(path)
        || matches!(
            path,
            ".codex-plugin/plugin.json"
                | "validator/Cargo.toml"
                | ".harness/coverage-manifest.json"
                | "templates/.harness/coverage-manifest.json"
        )
        || path.starts_with("schemas/")
        || path.starts_with("templates/")
        || path.starts_with("dev/observability/")
        || path.starts_with("validator/")
}

fn is_package_owned_source(path: &str) -> bool {
    path.starts_with("validator/")
        || path.starts_with("schemas/")
        || path.starts_with("templates/")
        || path.starts_with("skills/")
        || path.starts_with("agents/")
        || path.starts_with("custom-agents/")
        || path.starts_with("connectors/")
        || path.starts_with("dev/observability/")
        || path.starts_with("scripts/")
        || path.starts_with("docs/")
        || path.starts_with("artifacts/")
        || path.starts_with("examples/")
        || path.starts_with("install/")
        || path.starts_with(".cargo/")
        || path.starts_with(".codex/")
        || path.starts_with(".harness/")
        || path.starts_with(".codex-plugin/")
        || path.starts_with("fixtures/")
        || is_package_root_resource(path)
}

fn is_validation_artifact(path: &str) -> bool {
    path.starts_with("validation_artifacts/")
}

fn is_package_inventory_closure_input(path: &str) -> bool {
    // Package inventory enforces closure, so any product-candidate path can be
    // the exact new file the inventory must reject when it is not registered.
    !is_validation_artifact(path)
}

fn is_package_root_resource(path: &str) -> bool {
    matches!(
        path,
        ".gitignore"
            | "Cargo.lock"
            | "Cargo.toml"
            | "README.md"
            | "REPORT.md"
            | "audit.toml"
            | "deny.toml"
            | "package.json"
            | "plugin-manifest-draft.json"
            | "pnpm-lock.yaml"
            | "pnpm-workspace.yaml"
            | "rust-toolchain.toml"
    )
}

fn is_audit_context_input(path: &str) -> bool {
    is_rust_source(path)
        || path.starts_with("schemas/")
        || path.starts_with("fixtures/")
        || path.starts_with("templates/agent-standards/")
        || path.starts_with("docs/research-")
        || path.starts_with("docs/foundational-")
        || path.starts_with("docs/source-obligation")
        || path.starts_with("docs/mandatory-law")
}

fn is_observability_input(path: &str) -> bool {
    path.starts_with("validator/src/audit/observability/")
        || path.starts_with("validator/src/cli/observe/")
        || path.starts_with("validator/src/cli/live_loop/")
        || path.starts_with("schemas/observability-")
        || path.starts_with("docs/generated/observability/")
        || path.starts_with("dev/observability/")
        || path.starts_with("validation_artifacts/observability/")
}

fn is_law_input(path: &str) -> bool {
    path.starts_with("templates/agent-standards/")
        || path.starts_with("docs/research-")
        || path.starts_with("docs/foundational-")
        || path.starts_with("docs/source-obligation")
        || path.starts_with("docs/mandatory-law")
        || path.starts_with("validator/src/audit/mandatory/")
        || path.starts_with("validator/src/audit/law/")
}

fn is_receipt_input(path: &str) -> bool {
    path.starts_with("fixtures/")
        || path.starts_with("schemas/")
        || is_validation_artifact(path)
        || path.starts_with("validator/src/audit/receipt/")
        || path.starts_with("validator/src/audit/observability/registry/")
}

fn is_source_obligation_input(path: &str) -> bool {
    path.starts_with("docs/source-obligation")
        || path.starts_with("validator/src/audit/source_obligations")
        || path.starts_with("templates/agent-standards/")
}

fn is_foundational_trace_input(path: &str) -> bool {
    path.starts_with("docs/foundational-")
        || path.starts_with("docs/research-")
        || path.starts_with("validator/src/audit/foundational")
        || path.starts_with("validator/src/cli/foundational_trace")
}

fn is_coverage_input(path: &str) -> bool {
    is_rust_source(path)
        || path == ".harness/coverage-manifest.json"
        || path == "templates/.harness/coverage-manifest.json"
        || path.starts_with("validation_artifacts/coverage/")
        || path.starts_with("scripts/check-coverage")
        || path.starts_with("validator/tests/")
        || path.starts_with("validator/src/bin/")
}

fn is_fixture_input(path: &str) -> bool {
    path.starts_with("fixtures/")
        || path.starts_with("validator/src/audit/red/")
        || path.starts_with("validator/src/self_tests/red/")
        || path.starts_with("validator/src/self_tests/")
}

fn is_scripts_check_input(path: &str) -> bool {
    path == "scripts/check"
        || path.starts_with("scripts/check-")
        || is_rust_source(path)
        || is_rust_build_input(path)
        || path.starts_with("schemas/")
}

fn is_rust_source(path: &str) -> bool {
    path.starts_with("validator/") && path.ends_with(".rs")
}

fn is_rust_format_config(path: &str) -> bool {
    matches!(
        path,
        "rustfmt.toml" | ".rustfmt.toml" | "validator/rustfmt.toml" | "validator/.rustfmt.toml"
    )
}

fn is_rust_build_input(path: &str) -> bool {
    matches!(path, "Cargo.toml" | "Cargo.lock" | "validator/Cargo.toml")
}

fn is_json_surface(path: &str) -> bool {
    path.ends_with(".json") || path.ends_with(".jsonl")
}
