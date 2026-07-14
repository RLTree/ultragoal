use super::scenario::*;
use serde_json::json;
use std::fs;
use std::net::TcpListener;
use ultragoal::observability::EventStore;

#[test]
fn fresh_reads_and_configured_export_refusal_are_repeatable_zero_write() {
    let fresh = Repository::new("fresh", false, false);
    let _ = assert_payload_zero_write(
        &fresh,
        &["--json", "--help"],
        &[],
        &[0],
        "harness-ultragoal.cli-help.v1",
    );
    let (_, query_value) = assert_payload_zero_write(
        &fresh,
        &["--json", "observe", "query"],
        &[],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(query_value["store_status"], "absent");
    assert_eq!(query_value["event_count"], 0);
    assert_eq!(query_value["query_provenance"]["read_effect"], "none");
    let (_, diagnosis) = assert_payload_zero_write(
        &fresh,
        &["--json", "diagnose"],
        &[],
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(diagnosis["observability"]["store_status"], "not_opened");
    assert_eq!(diagnosis["observability"]["claim_effect"], "none");
    assert!(!fresh.store_path().exists());

    let configured = Repository::new("configured-export", true, true);
    let finding = selected_finding(&configured);
    let baseline_query = configured.run(&["--json", "observe", "query"]);
    let baseline_diagnosis =
        configured.run(&["--json", "diagnose", "--finding", &finding.finding_id]);
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let environment = [
        ("HTTP_PROXY", endpoint.as_str()),
        ("HTTPS_PROXY", endpoint.as_str()),
        ("ALL_PROXY", endpoint.as_str()),
        ("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint.as_str()),
        ("ULTRAGOAL_OBSERVABILITY_EXPORT_ENDPOINT", endpoint.as_str()),
    ];
    let (configured_query, query) = assert_payload_zero_write(
        &configured,
        &["--json", "observe", "query"],
        &environment,
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(configured_query.stdout, baseline_query.stdout);
    assert_eq!(query["local_policy"]["mode"], "local-only");
    assert_eq!(
        query["local_policy"]["configured_export_on_read"],
        "refused-no-external-effect"
    );
    let (configured_diagnosis, _) = assert_payload_zero_write(
        &configured,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &environment,
        &[1],
        "ProductStateDiagnose-v1",
    );
    assert_eq!(configured_diagnosis.stdout, baseline_diagnosis.stdout);
    let before_export = observe(configured.root());
    let export = configured.run_with_env(
        &[
            "--json",
            "observe",
            "export",
            "--output",
            "journey-export.json",
            "--approve-export",
        ],
        &environment,
    );
    assert_eq!(export.status.code(), Some(3), "{export:?}");
    assert!(export.stdout.is_empty(), "{export:?}");
    assert_eq!(
        machine(&export.stderr)["diagnostic_id"],
        "successor_runtime_authority_required"
    );
    assert_private_absent(&export, configured.root());
    assert!(!combined_text(&export).contains(&endpoint));
    assert!(!configured.root().join("journey-export.json").exists());
    assert_eq!(observe(configured.root()), before_export);
    assert_no_connection(&listener);
}

#[test]
fn diagnosis_reports_latest_bounded_failure_provenance_without_false_pass() {
    let repository = Repository::new("causal", true, true);
    let binding = binding(&repository);
    let finding = selected_finding(&repository);
    let store = open_store(&repository, &binding);
    let root = event(&binding, "root-failure", 1, "command.prepare", "fail");
    let mut target = event(&binding, "target-failure", 2, "command.execute", "blocked");
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
    let mut success = event(&binding, "newer-success", 3, "command.execute", "pass");
    success.add_finding_ref(&finding.finding_id).unwrap();
    let mut receipt = event(&binding, "newest-receipt", 4, "receipt.command", "fail");
    receipt.add_finding_ref(&finding.finding_id).unwrap();
    for item in [&root, &target, &success, &receipt] {
        assert!(store.append(item).unwrap());
    }
    let persisted = fs::read_to_string(repository.store_path()).unwrap();
    for private in [PRIVATE_TOKEN, PRIVATE_PATH, PRIVATE_EMAIL] {
        assert!(!persisted.contains(private));
    }
    assert!(persisted.contains("bounded-public-value"));

    let (_, query) = assert_payload_zero_write(
        &repository,
        &["--json", "observe", "query"],
        &[],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(query["event_count"], 4);
    assert_eq!(query["query_provenance"]["result_window_saturated"], false);
    let (_, diagnosis) = assert_payload_zero_write(
        &repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &[],
        &[1],
        "ProductStateDiagnose-v1",
    );
    let observed = &diagnosis["observability"];
    assert_eq!(observed["schema_version"], "PublicCausalDiagnosis-v1");
    assert_eq!(observed["matched_event_id"], "target-failure");
    assert_eq!(observed["failure_provenance"]["matched_reference_count"], 3);
    assert_eq!(observed["failure_provenance"]["eligible_failure_count"], 1);
    assert_eq!(
        observed["failure_provenance"]["selected"],
        json!({
            "event_id": "target-failure",
            "parent_event_id": "root-failure",
            "source_id": SOURCE_ID,
            "operation": "command.execute",
            "outcome": "blocked",
            "observed_at_unix_ms": 2,
            "sequence": 2,
            "causal_failure_eligible": true
        })
    );
    assert_eq!(observed["explanation"]["classification"], "observed-cause");
    assert_eq!(
        observed["explanation"]["causal_event_ids"],
        json!(["root-failure", "target-failure"])
    );
    assert!(
        observed["explanation"]["repair"]
            .as_str()
            .is_some_and(|value| value.contains("Apply the repair"))
    );
    assert_eq!(observed["claim_effect"], "none");
    assert_eq!(diagnosis["claim_effect"], "none");
}

#[test]
fn saturated_public_window_withholds_global_cause() {
    let repository = Repository::new("saturated", true, false);
    let binding = binding(&repository);
    let finding = selected_finding(&repository);
    let store = open_store(&repository, &binding);
    let limit = EventStore::supported_result_limit();
    for index in 0..limit {
        let mut item = event(
            &binding,
            &format!("bounded-{index:04}"),
            index as u64 + 1,
            "bounded.operation",
            if index + 1 == limit { "fail" } else { "pass" },
        );
        if index + 1 == limit {
            item.add_finding_ref(&finding.finding_id).unwrap();
        }
        assert!(store.append(&item).unwrap());
    }
    let (_, query) = assert_payload_zero_write(
        &repository,
        &["--json", "observe", "query"],
        &[],
        &[0],
        "ObservabilityQuery-v1",
    );
    assert_eq!(query["event_count"], limit);
    assert_eq!(query["query_provenance"]["result_window_saturated"], true);
    let (_, diagnosis) = assert_payload_zero_write(
        &repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &[],
        &[1],
        "ProductStateDiagnose-v1",
    );
    let observed = &diagnosis["observability"];
    assert!(observed["matched_event_id"].is_null());
    assert_eq!(observed["query_window"]["saturated"], true);
    assert_eq!(
        observed["explanation"]["classification"],
        "incomplete-evidence"
    );
    assert_eq!(
        observed["explanation"]["diagnostic_code"],
        "observe-evidence-incomplete:query-result-limit"
    );
    assert_eq!(observed["claim_effect"], "none");
}
