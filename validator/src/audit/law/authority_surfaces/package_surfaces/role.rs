pub(super) fn product_role_from_path(rel: &str) -> String {
    let role = if rel.starts_with("validator/src/argument_parser/") {
        "CLI argument parsing and typed command authority conversion"
    } else if rel == "validator/src/command/mod.rs" {
        "CLI execution command projection from typed argument authority"
    } else if rel == "validator/src/command_run.rs" {
        "CLI command dispatch and product behavior execution"
    } else if rel.starts_with("validator/src/audit/law/authority_surfaces/package_surfaces/") {
        "package active-surface inventory and product-role enforcement"
    } else if rel.starts_with("validator/src/audit/law/authority_surfaces/source/raw/") {
        "raw authority boundary classification and typed-failure reporting"
    } else if rel.starts_with("validator/src/audit/namespace/")
        || rel.starts_with("validator/src/cli/namespace/")
    {
        "semantic namespace law enforcement"
    } else if rel.starts_with("validator/src/cli/live_loop/") {
        "verified local repair-loop execution and node status reporting"
    } else if rel.starts_with("validator/src/cli/observe/") {
        "command telemetry query and repair explanation"
    } else if rel.starts_with("validator/src/cli/control/plane/") {
        "claim control-plane proof and refusal execution"
    } else if rel.starts_with("validator/src/cli/package/inventory/") {
        "package truth inventory and boundary reconciliation"
    } else if rel.starts_with("validator/src/cli/coverage/") {
        "coverage receipt verification and dead-code law enforcement"
    } else if rel.starts_with("validator/src/cli/") {
        "CLI command product behavior surface"
    } else if rel.starts_with("validator/src/audit/") {
        "source audit law enforcement"
    } else if rel.starts_with("validator/src/package/") {
        "package boundary truth calculation"
    } else if rel.starts_with("validator/src/self_tests/") || rel.contains("/tests/") {
        "test fixture validation for product law enforcement"
    } else if rel.starts_with("fixtures/red/") {
        "red fixture catalog for fail-closed law behavior"
    } else if rel.starts_with("fixtures/") {
        "fixture catalog for validation behavior"
    } else if rel.starts_with("schemas/") {
        "schema contract for typed artifact parsing"
    } else if rel.starts_with("docs/generated/") || rel.starts_with("examples/generated/") {
        "generated product-state projection artifact"
    } else if rel.starts_with("scripts/") || rel.starts_with("templates/scripts/") {
        "script delegation surface governed by canonical CLI"
    } else {
        "package-owned product behavior surface"
    };
    role.to_string()
}

pub(super) fn weak_product_role(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let bad_goal_label = ["production", "proof"].join("_");
    value.trim().is_empty()
        || lower.starts_with("rust module ")
        || lower.contains("fitting")
        || lower.contains(&bad_goal_label)
        || lower.contains("phase4")
        || lower.contains("checkpoint")
        || lower.contains("row-shape")
        || lower.contains("row_shape")
}
