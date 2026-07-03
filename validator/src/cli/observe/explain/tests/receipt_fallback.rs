use serde_json::json;
use std::fs;

#[test]
fn explain_fails_closed_when_matching_receipt_has_no_event_binding() {
    let root =
        crate::self_tests::boundaries::support::temp_root("observe-explain-receipt-fallback");
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("owned.txt"), "owned").expect("owned");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["owned.txt"]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("digest");
    let dir = root.join("validation_artifacts/observability");
    fs::create_dir_all(&dir).expect("observability dir");
    crate::json_boundary::write_json(
        &dir.join("target-without-event.json"),
        &json!({
            "schema": crate::cli::observe::types::RECEIPT_SCHEMA,
            "run_id": "run-no-event",
            "candidate_digest": candidate,
            "operation": "coverage.prove",
            "status": "pass",
            "law_id": "coverage-self-law",
            "check_id": "coverage-prove",
            "claim_id": "coverage-claim"
        }),
    )
    .expect("receipt without event");
    let command = crate::cli::observe::parse(&[
        "observe".to_string(),
        "explain-failure".to_string(),
        "--run-id".to_string(),
        "run-no-event".to_string(),
    ])
    .expect("parse")
    .expect("observe");

    let receipt = super::run(&root, &command).expect("explain receipt fallback");

    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["failure_class"],
        "receipt_without_observability_event"
    );
    assert_eq!(
        receipt["where_failed"],
        "observe.target.receipt_event_binding"
    );
    assert_eq!(
        receipt["claim_impact"],
        "observability_reconciliation_blocked"
    );
    assert_eq!(receipt["explanation"]["fallback_used"], true);
    assert!(
        receipt["explanation"]["root_cause"]
            .as_str()
            .unwrap()
            .contains("has no event object")
    );
    assert!(
        receipt["explanation"]["smallest_repair"]
            .as_str()
            .unwrap()
            .contains("real event emission")
    );
    let sources = receipt["explanation"]["evidence_sources"]
        .as_array()
        .expect("evidence sources");
    assert!(sources.iter().any(|source| {
        source.as_str() == Some("target observability receipt without event binding")
    }));
    assert!(
        !sources
            .iter()
            .any(|source| source.as_str() == Some("target telemetry event"))
    );
    fs::remove_dir_all(root).expect("cleanup receipt fallback");
}
