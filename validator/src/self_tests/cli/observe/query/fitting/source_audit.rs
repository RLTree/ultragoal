#[test]
fn source_audit_inventory_remains_partial_until_proof_is_non_circular() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let inventory = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .expect("command inventory");
    let row = inventory
        .pointer("/fitting_inventory/source audit")
        .expect("source audit row");
    assert_eq!(row["fitting_status"], "partially_fitted");
    assert_eq!(
        row["next_unfitted_surface"],
        "stable same-run production proof that is not overwritten by the validating source-audit run"
    );
    assert!(
        row["missing_surfaces"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item
                .as_str()
                .is_some_and(|text| text.contains("not overwritten"))),
        "{row}"
    );
    let board = inventory
        .pointer("/fitting_control_board/first_incomplete")
        .expect("first incomplete");
    assert_eq!(board["id"], "source audit");
    assert_eq!(board["fitting_status"], "partially_fitted");
}
