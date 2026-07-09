use super::{inventory_row, missing_surfaces};
use serde_json::json;

#[test]
fn inventory_row_routes_each_control_board_family_to_product_inventory() {
    let inventory = json!({
        "command_observability_inventory": {"row": {"family": "commands"}},
        "surface_inventory": {"row": {"family": "surfaces"}},
        "operating_loop_inventory": {"row": {"family": "operating_loop"}},
        "signal_inventory": {"row": {"family": "signals"}},
        "validator_check_inventory": {"row": {"family": "validator_checks"}},
        "receipt_proof_inventory": {"row": {"family": "receipts"}},
        "fixture_report_inventory": {"row": {"family": "fixtures"}},
        "package_plugin_setup_retrofit_inventory": {"row": {"family": "package_setup"}},
        "long_running_path_inventory": {"row": {"family": "long_running"}},
        "external_live_path_inventory": {"row": {"family": "external_live"}},
        "claim_guard_inventory": {"row": {"family": "claim_guards"}}
    });

    for family in [
        "commands",
        "surfaces",
        "operating_loop",
        "signals",
        "validator_checks",
        "receipts",
        "fixtures",
        "package_setup",
        "long_running",
        "external_live",
        "claim_guards",
    ] {
        assert_eq!(inventory_row(&inventory, family, "row")["family"], family);
    }
    assert!(inventory_row(&inventory, "unknown", "row").is_null());
}

#[test]
fn missing_surfaces_falls_back_to_first_incomplete_next_surface() {
    let missing = missing_surfaces(
        &json!({}),
        &json!({"next_unobservable_surface": "same-candidate trace proof"}),
    );

    assert_eq!(missing, vec!["same-candidate trace proof".to_string()]);
    assert!(missing_surfaces(&json!({}), &json!({})).is_empty());
}
