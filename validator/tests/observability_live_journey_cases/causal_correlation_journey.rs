use super::*;

#[test]
pub(crate) fn public_query_and_diagnosis_preserve_redacted_stable_causal_correlation() {
    let repository = JourneyRepository::new("causal", true, true);
    let binding = public_binding(&repository);
    let finding = selected_finding(&repository);
    let store = open_store(&repository, &binding);

    let root = event(&binding, "journey-root", 1, "command.prepare");
    let mut target = event(&binding, "journey-target", 2, "command.execute");
    target.set_parent(root.event_id()).unwrap();
    target.add_finding_ref(&finding.finding_id).unwrap();
    target.add_repair_ref(&finding.repair_id).unwrap();
    target
        .add_public_attribute("api_token", PRIVATE_TOKEN)
        .unwrap();
    target
        .add_public_attribute("private_path", PRIVATE_PATH)
        .unwrap();
    target.add_public_attribute("owner", PRIVATE_EMAIL).unwrap();
    target
        .add_public_attribute("safe", "bounded-public-value")
        .unwrap();
    assert!(store.append(&root).unwrap());
    assert!(store.append(&target).unwrap());
    assert!(
        !store.append(&target).unwrap(),
        "exact append must be idempotent"
    );

    let persisted = fs::read_to_string(repository.store_path()).unwrap();
    for private in [PRIVATE_TOKEN, PRIVATE_PATH, PRIVATE_EMAIL] {
        assert!(!persisted.contains(private), "persisted {private}");
    }
    assert!(persisted.contains("bounded-public-value"));

    let query_value = assert_payload_repeat_zero_write(
        &repository,
        &["--json", "observe", "query", "--filter", "command.execute"],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(query_value["context_id"], binding.context_id);
    assert_eq!(query_value["candidate_id"], binding.candidate_id);
    assert_eq!(query_value["source_id"], binding.source_id);
    assert_eq!(query_value["event_count"], 1);
    assert_eq!(query_value["events"][0]["event_id"], "journey-target");
    assert_eq!(query_value["events"][0]["parent_event_id"], "journey-root");
    assert_eq!(query_value["claim_effect"], "none");
    assert_eq!(
        query_value["local_policy"]["external_export"],
        "disabled-safe-default-OD-004-OD-007"
    );
    assert_eq!(
        query_value["local_policy"]["store_limit_bytes"],
        EventStore::supported_store_limit_bytes()
    );
    assert_eq!(
        query_value["local_policy"]["event_limit"],
        EventStore::supported_event_limit()
    );
    assert_eq!(
        query_value["local_policy"]["scan_row_limit"],
        EventStore::supported_scan_limit()
    );
    assert_eq!(
        query_value["local_policy"]["query_result_limit"],
        EventStore::supported_result_limit()
    );
    assert_eq!(
        query_value["local_policy"]["lock_timeout_millis"],
        EventStore::supported_lock_timeout_millis()
    );

    let diagnosis = assert_payload_repeat_zero_write(
        &repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(diagnosis["observability"]["store_status"], "available");
    assert_eq!(
        diagnosis["observability"]["matched_event_id"],
        "journey-target"
    );
    assert_eq!(
        diagnosis["observability"]["explanation"]["classification"],
        "observed-cause"
    );
    assert_eq!(
        diagnosis["observability"]["explanation"]["causal_event_ids"],
        serde_json::json!(["journey-root", "journey-target"])
    );
    assert!(
        diagnosis["observability"]["explanation"]["repair"]
            .as_str()
            .is_some_and(|repair| repair.contains("Apply the repair"))
    );
    assert!(diagnosis["repairs"].as_array().is_some_and(|repairs| {
        repairs
            .iter()
            .any(|row| row["repair_id"].as_str() == Some(finding.repair_id.as_str()))
    }));
    assert_eq!(diagnosis["claim_effect"], "none");
    repository.teardown();
}

#[test]
pub(crate) fn receipt_only_event_cannot_false_pass_as_a_public_cause() {
    let repository = JourneyRepository::new("receipt-only", true, false);
    let binding = public_binding(&repository);
    let finding = selected_finding(&repository);
    let store = open_store(&repository, &binding);
    let mut receipt = event(&binding, "receipt-only-event", 1, "receipt.command");
    receipt.add_finding_ref(&finding.finding_id).unwrap();
    receipt.add_repair_ref(&finding.repair_id).unwrap();
    assert!(store.append(&receipt).unwrap());

    let diagnosis = assert_payload_repeat_zero_write(
        &repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(
        diagnosis["observability"]["matched_event_id"],
        "receipt-only-event"
    );
    assert_eq!(
        diagnosis["observability"]["explanation"]["classification"],
        "missing-evidence"
    );
    assert_eq!(
        diagnosis["observability"]["explanation"]["diagnostic_code"],
        "observe-evidence-missing:receipt-only"
    );
    assert_eq!(diagnosis["claim_effect"], "none");
    repository.teardown();
}

#[test]
pub(crate) fn stale_unknown_and_truncated_stores_fail_closed_and_only_explicit_recovery_writes() {
    let stale = JourneyRepository::new("stale-candidate", false, false);
    let stale_binding = public_binding(&stale);
    let stale_store = open_store(&stale, &stale_binding);
    assert!(
        stale_store
            .append(&event(&stale_binding, "stale-event", 1, "check.run"))
            .unwrap()
    );
    fs::write(stale.root().join("tracked.txt"), b"new candidate bytes\n").unwrap();
    assert_diagnostic_zero_write(
        &stale,
        &["--json", "observe", "query"],
        4,
        "successor_runtime_observability_unavailable",
    );

    let unknown = JourneyRepository::new("unknown-row", true, false);
    let unknown_binding = public_binding(&unknown);
    let unknown_finding = selected_finding(&unknown);
    let unknown_store = open_store(&unknown, &unknown_binding);
    assert!(
        unknown_store
            .append(&event(
                &unknown_binding,
                "unknown-row-event",
                1,
                "check.run"
            ))
            .unwrap()
    );
    let row = fs::read_to_string(unknown.store_path()).unwrap();
    fs::write(
        unknown.store_path(),
        row.replacen("}\n", ",\"unknown_row\":true}\n", 1),
    )
    .unwrap();
    assert_diagnostic_zero_write(
        &unknown,
        &["--json", "observe", "query"],
        4,
        "successor_runtime_observability_unavailable",
    );
    let corrupt_diagnosis = assert_payload_repeat_zero_write(
        &unknown,
        &[
            "--json",
            "diagnose",
            "--finding",
            &unknown_finding.finding_id,
        ],
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(
        corrupt_diagnosis["observability"]["explanation"]["classification"],
        "corruption"
    );

    let recovery = JourneyRepository::new("truncated-recovery", false, false);
    let recovery_binding = public_binding(&recovery);
    let recovery_store = open_store(&recovery, &recovery_binding);
    assert!(
        recovery_store
            .append(&event(&recovery_binding, "retained-event", 1, "check.run"))
            .unwrap()
    );
    OpenOptions::new()
        .append(true)
        .open(recovery.store_path())
        .unwrap()
        .write_all(b"{\"row_version\":\"SemanticEventRow-v1\"")
        .unwrap();
    assert_diagnostic_zero_write(
        &recovery,
        &["--json", "observe", "query"],
        4,
        "successor_runtime_observability_unavailable",
    );
    let status_before_recovery = git_status(recovery.root());
    let removed = recovery_store.recover_truncated_tail().unwrap();
    assert!(removed > 0);
    assert_eq!(git_status(recovery.root()), status_before_recovery);
    assert_eq!(recovery_store.recover_truncated_tail().unwrap(), 0);
    let recovered = assert_payload_repeat_zero_write(
        &recovery,
        &["--json", "observe", "query"],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(recovered["event_count"], 1);
    assert_eq!(recovered["events"][0]["event_id"], "retained-event");
    stale.teardown();
    unknown.teardown();
    recovery.teardown();
}
