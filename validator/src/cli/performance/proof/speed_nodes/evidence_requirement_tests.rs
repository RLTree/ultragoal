use super::*;

#[test]
fn executed_speed_rows_need_receipt_or_artifact_path() {
    let candidate = test_rows::digest('a');
    let mut row = test_rows::executed_row(&candidate);
    row["receipt_paths"] = json!([]);
    row["artifact_paths"] = json!([]);

    assert!(super::super::claim_readiness::node_blocks_speed_claim(&row));
}

#[test]
fn executed_speed_rows_need_positive_actual_work_latency() {
    let candidate = test_rows::digest('a');
    let mut row = test_rows::executed_row(&candidate);
    row["actual_work_duration_ms"] = json!(0);

    assert!(super::super::claim_readiness::node_blocks_speed_claim(&row));
}

#[test]
fn test_projection_rejects_missing_product_latency_inputs_directly() {
    let candidate = test_rows::digest('a');
    for field in [
        "actual_work_duration_ms",
        "graph_overhead_ms",
        "telemetry_reconciliation_duration_ms",
        "reconciled_command_duration_ms",
        "product_latency_ms",
    ] {
        let mut row = test_rows::executed_row(&candidate);
        row.as_object_mut().expect("row object").remove(field);
        assert!(
            !super::super::row_has_reconciled_product_latency(&row),
            "{field} must be required"
        );
    }

    let mut zero_actual = test_rows::executed_row(&candidate);
    zero_actual["actual_work_duration_ms"] = json!(0);
    assert!(!super::super::row_has_reconciled_product_latency(
        &zero_actual
    ));
}
