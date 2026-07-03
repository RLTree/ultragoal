#[test]
fn production_inventory_rows_account_for_required_observability_contracts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("production command inventory");
    for (family, key) in inventory_families() {
        let rows = inventory
            .get(key)
            .and_then(serde_json::Value::as_object)
            .expect("inventory rows");
        for (id, row) in rows {
            let object = row.as_object().expect("inventory row object");
            assert!(
                super::super::super::row_contract::complete(object),
                "{family}:{id} must account for log, metric, trace, stdout, and receipt binding"
            );
        }
    }
}

fn inventory_families() -> [(&'static str, &'static str); 11] {
    [
        ("commands", "command_observability_inventory"),
        ("surfaces", "surface_inventory"),
        ("operating_loop", "operating_loop_inventory"),
        ("signals", "signal_inventory"),
        ("validator_checks", "validator_check_inventory"),
        ("receipts", "receipt_proof_inventory"),
        ("fixtures", "fixture_report_inventory"),
        ("package_setup", "package_plugin_setup_retrofit_inventory"),
        ("long_running", "long_running_path_inventory"),
        ("external_live", "external_live_path_inventory"),
        ("claim_guards", "claim_guard_inventory"),
    ]
}
