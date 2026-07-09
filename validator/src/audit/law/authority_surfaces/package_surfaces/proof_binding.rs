use super::{role, row::PackageSurfaceRow};

pub(super) fn contract_failure(row: &PackageSurfaceRow) -> Option<&'static str> {
    if role::weak_product_role(&row.product_role) {
        return Some("surface_product_role_is_goal_or_path_shape_not_product_behavior");
    }
    if row.authority_level == "compatibility_alias" && row.compatibility_contract_id.is_none() {
        return Some("compatibility_alias_missing_external_contract");
    }
    if row.authority_level == "compatibility_alias" && row.sunset_condition.is_none() {
        return Some("compatibility_alias_missing_sunset_removal_rule");
    }
    if super::row::claim_surfaces(&row.proof_surface).is_empty() {
        return None;
    }
    if fixture_ids(row).is_empty() {
        return Some("surface_missing_red_green_tamper_fixture_binding");
    }
    if receipt_ids(row).is_empty() {
        return Some("surface_missing_receipt_or_inventory_reconciliation_binding");
    }
    None
}

pub(super) fn fixture_ids(row: &PackageSurfaceRow) -> Vec<&'static str> {
    match row.authority_level.as_str() {
        "test_only_validation_surface" => {
            vec!["validator-theater-miswire-resistance-row-shape-only-mechanization-red"]
        }
        "parser_boundary" => vec!["typed-records-over-prose-raw-path-authority-rejected-red"],
        "fixture_catalog_materialization" => {
            vec!["validator-theater-miswire-resistance-row-shape-only-mechanization-red"]
        }
        "generated_projection" => {
            vec!["validator-theater-miswire-resistance-row-shape-only-mechanization-red"]
        }
        "external_debug_no_claim" => Vec::new(),
        "compatibility_alias" => vec!["namespace-external-compatibility-authority-missing"],
        _ => vec![
            "namespace-validator-source-generated-class-red",
            "distinct-proof-surfaces-claim-ceilings-wrong-proof-surface-red",
        ],
    }
}

pub(super) fn receipt_ids(row: &PackageSurfaceRow) -> Vec<&'static str> {
    match row.authority_level.as_str() {
        "external_debug_no_claim" => Vec::new(),
        "test_only_validation_surface" => {
            vec!["validation_artifacts/ultragoal-audit/red-fixture-report.json"]
        }
        "compatibility_alias" if row.compatibility_contract_id.is_none() => Vec::new(),
        _ => vec![
            "validation_artifacts/package/inventory/package-surface-inventory.json",
            "validation_artifacts/ultragoal-audit/validator-receipt.json",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::super::row;

    #[test]
    fn package_surface_rows_reject_goal_label_roles() {
        let mut row = row::from_parts(
            "function",
            &format!(
                "validator/src/cli/observe/command_roundtrip.rs::{}",
                ["production", "proof"].join("_")
            ),
            &["production", "proof"].join("_"),
            "canonical",
            None,
        );
        assert_eq!(
            super::contract_failure(&row),
            Some("surface_product_role_is_goal_or_path_shape_not_product_behavior")
        );

        row.product_role = "command telemetry roundtrip reconciliation".to_string();
        assert_eq!(super::contract_failure(&row), None);
    }

    #[test]
    fn claim_bearing_rows_carry_fixture_and_receipt_bindings() {
        let row = row::source_module("validator/src/cli/observe/query/mod.rs", false);
        let value = row::value(row);
        assert!(
            value["fixture_ids"]
                .as_array()
                .is_some_and(|items| !items.is_empty()),
            "{value}"
        );
        assert!(
            value["receipt_ids"]
                .as_array()
                .is_some_and(|items| !items.is_empty()),
            "{value}"
        );
        assert_eq!(
            value["provenance"]["manual_edit_status"],
            "generated_rows_must_be_recomputed_not_hand_edited"
        );
    }

    #[test]
    fn claim_bearing_rows_fail_without_fixture_or_receipt_reconciliation() {
        let mut row = row::source_module("validator/src/cli/observe/query/mod.rs", false);
        row.authority_level = "external_debug_no_claim".to_string();
        assert_eq!(
            super::contract_failure(&row),
            Some("surface_missing_red_green_tamper_fixture_binding")
        );

        row = row::from_parts(
            "binary",
            "validator/src/bin/ultragoal-validator.rs",
            "compatibility CLI executable alias",
            "compatibility_alias",
            Some("binary:ultragoal"),
        );
        assert_eq!(
            super::contract_failure(&row),
            Some("compatibility_alias_missing_external_contract")
        );

        row.compatibility_contract_id = Some("legacy-ultragoal-validator-cli".to_string());
        assert_eq!(
            super::contract_failure(&row),
            Some("compatibility_alias_missing_sunset_removal_rule")
        );

        row.sunset_condition =
            Some("remove after downstream callers migrate to ultragoal".to_string());
        assert_eq!(super::contract_failure(&row), None);
    }
}
