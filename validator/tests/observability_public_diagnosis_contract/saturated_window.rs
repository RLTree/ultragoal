use super::scenario::*;
use ultragoal::observability::EventStore;

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
    repository.teardown();
}
