use super::{proof_binding, role, source::SourceSymbol};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageSurfaceRow {
    pub(crate) surface_id: String,
    pub(crate) surface_kind: String,
    pub(crate) path_or_symbol: String,
    pub(crate) product_role: String,
    pub(crate) canonical_owner: String,
    pub(crate) authority_level: String,
    pub(crate) canonical_surface_id: Option<String>,
    pub(crate) compatibility_contract_id: Option<String>,
    pub(crate) sunset_condition: Option<String>,
    pub(crate) proof_surface: String,
    pub(crate) package_inventory_binding: String,
}

pub(super) fn from_parts(
    kind: &str,
    path_or_symbol: &str,
    product_role: &str,
    authority_level: &str,
    canonical_surface_id: Option<&str>,
) -> PackageSurfaceRow {
    PackageSurfaceRow {
        surface_id: format!("{}:{}", sanitize(kind), sanitize(path_or_symbol)),
        surface_kind: kind.to_string(),
        path_or_symbol: path_or_symbol.to_string(),
        product_role: product_role.to_string(),
        canonical_owner: canonical_surface_id.unwrap_or(path_or_symbol).to_string(),
        authority_level: authority_level.to_string(),
        canonical_surface_id: canonical_surface_id.map(ToString::to_string),
        compatibility_contract_id: None,
        sunset_condition: None,
        proof_surface: proof_surface(authority_level).to_string(),
        package_inventory_binding: "derived_from_current_package_inventory".to_string(),
    }
}

pub(super) fn source_module(rel: &str, test_only: bool) -> PackageSurfaceRow {
    from_parts(
        "rust_module",
        rel,
        &role::product_role_from_path(rel),
        if test_only {
            "test_only_validation_surface"
        } else {
            authority_level_for_path(rel)
        },
        None,
    )
}

pub(super) fn source_symbol(
    rel: &str,
    symbol: &SourceSymbol,
    module_test_only: bool,
) -> PackageSurfaceRow {
    from_parts(
        symbol.kind,
        &format!("{rel}::{}", symbol.name),
        &format!(
            "{} supporting {}",
            symbol.kind,
            role::product_role_from_path(rel)
        ),
        if symbol.test_only || module_test_only {
            "test_only_validation_surface"
        } else {
            authority_level_for_path(rel)
        },
        None,
    )
}

pub(super) fn package_resource(rel: &str) -> PackageSurfaceRow {
    from_parts(
        surface_kind_for_path(rel),
        rel,
        &role::product_role_from_path(rel),
        authority_level_for_path(rel),
        None,
    )
}

pub(super) fn retained_context(rel: &str, replacement_targets: &[String]) -> PackageSurfaceRow {
    let mut row = from_parts(
        "retained_context",
        rel,
        "preserved predecessor context with adopted canonical replacements",
        "retained_context_no_claim",
        replacement_targets.first().map(String::as_str),
    );
    row.canonical_owner = replacement_targets.join(",");
    row
}

pub(super) fn invalid_generated_authority(rel: &str) -> PackageSurfaceRow {
    from_parts(
        "generated_artifact",
        rel,
        "generated package surface with unavailable authority disposition",
        "invalid_generated_authority",
        None,
    )
}

pub(super) fn contract_failure(row: &PackageSurfaceRow) -> Option<&'static str> {
    proof_binding::contract_failure(row)
}

pub(super) fn claim_surfaces(proof_surface: &str) -> Vec<&'static str> {
    match proof_surface {
        "test_validation" | "fixture_catalog" => vec!["validation"],
        "generated_projection" => vec!["source_local_projection"],
        "external_debug_no_claim" | "retained_context_no_claim" | "invalid_no_claim" => Vec::new(),
        _ => vec!["source_local"],
    }
}

fn surface_kind_for_path(rel: &str) -> &'static str {
    if rel.starts_with("schemas/") {
        "schema"
    } else if rel.starts_with("fixtures/red/") {
        "red_fixture"
    } else if rel.starts_with("fixtures/") {
        "fixture"
    } else if rel.starts_with("docs/generated/") || rel.starts_with("examples/generated/") {
        "generated_artifact"
    } else if rel.starts_with("validation_artifacts/") {
        "receipt_or_report"
    } else if rel.starts_with("scripts/")
        || rel.starts_with("templates/scripts/")
        || rel.starts_with(".harness/")
    {
        "script"
    } else if rel.contains("final-packet") {
        "final_packet_blocker"
    } else if rel.contains("update-goal") {
        "update_goal_blocker"
    } else {
        "package_resource"
    }
}

fn authority_level_for_path(rel: &str) -> &'static str {
    if rel.starts_with("validator/tests/")
        || rel.contains("/self_tests/")
        || rel.ends_with("_tests.rs")
        || rel.ends_with("/tests.rs")
        || rel.contains("/tests/")
    {
        "test_only_validation_surface"
    } else if rel.starts_with("schemas/")
        || rel == "validator/src/argument_parser.rs"
        || rel.starts_with("validator/src/argument_parser/")
    {
        "parser_boundary"
    } else if rel.starts_with("fixtures/") {
        "fixture_catalog_materialization"
    } else if rel.starts_with("docs/generated/") || rel.starts_with("examples/generated/") {
        "generated_projection"
    } else if rel.starts_with("validation_artifacts/") {
        "external_debug_no_claim"
    } else {
        "canonical"
    }
}

fn proof_surface(authority_level: &str) -> &'static str {
    match authority_level {
        "test_only_validation_surface" => "test_validation",
        "fixture_catalog_materialization" => "fixture_catalog",
        "generated_projection" => "generated_projection",
        "external_debug_no_claim" => "external_debug_no_claim",
        "retained_context_no_claim" => "retained_context_no_claim",
        "invalid_generated_authority" => "invalid_no_claim",
        _ => "source",
    }
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

pub(super) fn dead_surface_preservation_label(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("coverage_only")
        || lower.contains("coverage-only")
        || lower.contains("hit_count")
        || lower.contains("hit-count")
        || lower.contains("no_op")
        || lower.contains("noop")
}
