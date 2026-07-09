pub(super) fn is_non_product_input(path: &str) -> bool {
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

pub(super) fn always_product_path(path: &str) -> bool {
    !path.is_empty()
}

pub(super) fn is_package_owned_source(path: &str) -> bool {
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

pub(super) fn is_package_inventory_closure_input(path: &str) -> bool {
    !is_validation_artifact(path)
}

fn is_package_root_resource(path: &str) -> bool {
    matches!(
        path,
        ".gitignore"
            | "Cargo.lock"
            | "Cargo.toml"
            | "LICENSE"
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

pub(super) fn is_audit_context_input(path: &str) -> bool {
    is_rust_source(path)
        || path.starts_with("schemas/")
        || path.starts_with("fixtures/")
        || path.starts_with("templates/agent-standards/")
        || path.starts_with("docs/research-")
        || path.starts_with("docs/foundational-")
        || path.starts_with("docs/source-obligation")
        || path.starts_with("docs/mandatory-law")
}

pub(super) fn is_observability_input(path: &str) -> bool {
    path.starts_with("validator/src/audit/observability/")
        || path.starts_with("validator/src/cli/observe/")
        || path.starts_with("validator/src/cli/live_loop/")
        || path.starts_with("schemas/observability-")
        || path.starts_with("docs/generated/observability/")
        || path.starts_with("dev/observability/")
        || path.starts_with("validation_artifacts/observability/")
}

pub(super) fn is_mandatory_law_input(path: &str) -> bool {
    is_law_input(path) || is_receipt_input(path)
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

pub(super) fn is_source_obligation_input(path: &str) -> bool {
    path.starts_with("docs/source-obligation")
        || path.starts_with("validator/src/audit/source_obligations")
        || path.starts_with("templates/agent-standards/")
}

pub(super) fn is_foundational_trace_input(path: &str) -> bool {
    path.starts_with("docs/foundational-")
        || path.starts_with("docs/research-")
        || path.starts_with("validator/src/audit/foundational")
        || path.starts_with("validator/src/cli/foundational_trace")
}

pub(super) fn is_coverage_input(path: &str) -> bool {
    is_rust_source(path)
        || path == ".harness/coverage-manifest.json"
        || path == "templates/.harness/coverage-manifest.json"
        || path.starts_with("validation_artifacts/coverage/")
        || path.starts_with("scripts/check-coverage")
        || path.starts_with("validator/tests/")
        || path.starts_with("validator/src/bin/")
}

pub(super) fn is_fixture_input(path: &str) -> bool {
    path.starts_with("fixtures/")
        || path.starts_with("validator/src/audit/red/")
        || path.starts_with("validator/src/self_tests/red/")
        || path.starts_with("validator/src/self_tests/")
}

pub(super) fn is_scripts_check_input(path: &str) -> bool {
    path == "scripts/check"
        || path.starts_with("scripts/check-")
        || is_rust_source(path)
        || is_rust_build_input(path)
        || path.starts_with("schemas/")
}

pub(super) fn is_source_audit_input(path: &str) -> bool {
    is_package_inventory_closure_input(path)
        || is_package_boundary_input(path)
        || is_package_owned_source(path)
        || is_law_input(path)
        || is_receipt_input(path)
        || is_fixture_input(path)
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

pub(super) fn is_fmt_input(path: &str) -> bool {
    is_rust_source(path) || is_rust_format_config(path)
}

pub(super) fn is_rust_build_surface_input(path: &str) -> bool {
    is_rust_source(path) || is_rust_build_input(path)
}

pub(super) fn is_line_cap_input(path: &str) -> bool {
    is_rust_source(path)
        || (path.starts_with(".harness/") && path.ends_with(".sh"))
        || matches!(
            path,
            ".harness/coverage-command"
                | "scripts/check"
                | "scripts/check-agent-standards"
                | "scripts/check-coverage-fast"
                | "scripts/check-coverage-full"
        )
}

pub(super) fn is_namespace_input(path: &str) -> bool {
    is_rust_source(path)
        || path == "docs/namespace-law-exceptions.json"
        || path == "plugin-manifest-draft.json"
}

pub(super) fn is_schema_input(path: &str) -> bool {
    is_json_surface(path) || path.starts_with("schemas/")
}

pub(super) fn is_rust_source(path: &str) -> bool {
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
