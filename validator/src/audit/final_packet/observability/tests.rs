use serde_json::json;

fn root() -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root("final-packet-observability");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    root
}

#[test]
fn final_packet_observability_rejects_missing_wrong_and_empty_failure_receipts() {
    let root = root();
    let missing = super::failures(&root, &json!({"status":"fail"}));
    assert_eq!(
        missing,
        vec!["final_packet_proof_observability_missing".to_string()]
    );

    let bad = json!({
        "status": "fail",
        "observability": {
            "schema": "wrong",
            "status": "pass",
            "candidate_digest": crate::digest::ZERO,
            "run_id": "",
            "correlation_id": "",
            "why_failed": "none",
            "where_failed": "",
            "next_repair": "",
            "event": {"one": 1},
            "metric": {"two": 2},
            "trace": {"three": 3},
            "log_stream_digest": crate::digest::ZERO,
            "metric_snapshot_digest": crate::digest::ZERO,
            "trace_bundle_digest": crate::digest::ZERO
        }
    });
    let failures = super::failures(&root, &bad);
    assert!(failures.contains(&"final_packet_proof_observability_wrong_schema".to_string()));
    assert!(
        failures
            .contains(&"final_packet_proof_observability_candidate_digest_mismatch".to_string())
    );
    assert!(failures.contains(&"final_packet_proof_observability_status_mismatch".to_string()));
    assert!(failures.contains(&"final_packet_proof_observability_missing:run_id".to_string()));
    assert!(failures.contains(&"final_packet_proof_observability_empty_failure".to_string()));
    assert!(failures.iter().any(|failure| {
        failure.starts_with("final_packet_proof_observability_digest_mismatch:")
    }));

    let mut missing_subdoc = bad;
    missing_subdoc["observability"]
        .as_object_mut()
        .unwrap()
        .remove("event");
    let failures = super::failures(&root, &missing_subdoc);
    assert!(failures.contains(&"final_packet_proof_observability_missing:/event".to_string()));

    let event = json!({
        "run_id": "run-final-packet",
        "correlation_id": "corr-final-packet",
        "trace_id": "trace-final-packet",
        "span_id": "span-final-packet",
        "why_failed": "final_packet_proof_packet_absent",
        "where_failed": "final-packet.prove",
        "next_repair": "repair final packet dependency"
    });
    let metric = json!({"status":"fail"});
    let trace = json!({
        "trace_id": "trace-final-packet",
        "span_id": "span-final-packet",
        "child_spans": []
    });
    let missing_receipt_span = json!({
        "status": "fail",
        "cli_performance": {
            "path": "validation_artifacts/cli/performance-receipt.json"
        },
        "observability": {
            "schema": "harness-ultragoal.observability-receipt.v1",
            "status": "fail",
            "candidate_digest": crate::package::inventory::package_digest(&root).unwrap(),
            "run_id": "run-final-packet",
            "correlation_id": "corr-final-packet",
            "why_failed": "final_packet_proof_packet_absent",
            "where_failed": "final-packet.prove",
            "next_repair": "repair final packet dependency",
            "event": event,
            "metric": metric,
            "trace": trace,
            "log_stream_digest": crate::digest::canonical_json(&event),
            "metric_snapshot_digest": crate::digest::canonical_json(&metric),
            "trace_bundle_digest": crate::digest::canonical_json(&trace)
        }
    });
    let failures = super::failures(&root, &missing_receipt_span);
    assert!(failures.iter().any(|failure| {
        failure.starts_with("final_packet_proof_observability_receipt_span_missing:cli_performance")
    }));
    std::fs::remove_dir_all(root).expect("cleanup");
}
