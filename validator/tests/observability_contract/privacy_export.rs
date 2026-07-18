use super::observability::ExplicitExportRequest;
use super::scenario::{AdapterMode, MockAdapter, TestDir, event, query, store};
use std::fs;
use std::time::Duration;
#[test]
fn tokens_pii_paths_and_bypass_variants_are_dropped_before_persistence() {
    let dir = TestDir::new("redaction");
    let store = store(&dir);
    let mut event = event("redacted", 1, 1, "fail");
    let canaries = [
        ("api_token", "sk-super-secret-value"),
        ("Api-KeY", "plain-secret"),
        ("owner", "person@example.test"),
        ("location", "/Users/private/secret.txt"),
        ("auth", "BeArEr abcdefghijklmnop"),
        ("encoded", "token%3Dabcdef"),
        ("jwt", "abcdefgh.ijklmnop.qrstuvwx"),
        ("phone", "+1 (415) 555-1212"),
        ("owner", "Actual Person"),
        ("tmp_path", "/private/tmp/secret.txt"),
        ("home_hint", "~/private/secret.txt"),
        ("windows_hint", "C:\\Users\\private\\secret.txt"),
        ("uri_hint", "file:///private/secret.txt"),
        ("unc_hint", "\\\\private-host\\secret-share\\secret.txt"),
        ("oauth_hint", "gho_private_oauth_token"),
        ("provider_hint", "xoxb-private-token"),
        ("cloud_hint", "AKIAPRIVATEKEYVALUE"),
        ("ssn_hint", "123-45-6789"),
    ];
    for (key, value) in canaries {
        event.add_public_attribute(key, value).unwrap();
    }
    event
        .add_sensitive_attribute("private-note", "raw sensitive note")
        .unwrap();
    event.add_public_attribute("safe", "bounded-value").unwrap();
    store.append(&event).unwrap();

    let persisted = fs::read_to_string(dir.store_path()).unwrap();
    for forbidden in [
        "sk-super-secret-value",
        "plain-secret",
        "person@example.test",
        "/Users/private",
        "abcdefghijklmnop",
        "token%3D",
        "abcdefgh.ijklmnop.qrstuvwx",
        "415",
        "raw sensitive note",
        "Actual Person",
        "/private/tmp",
        "~/private",
        "C:\\Users",
        "file:///private",
        "private-host\\secret-share",
        "gho_private_oauth_token",
        "xoxb-private-token",
        "AKIAPRIVATEKEYVALUE",
        "123-45-6789",
    ] {
        assert!(
            !persisted.contains(forbidden),
            "persisted canary {forbidden}"
        );
    }
    assert!(persisted.contains("bounded-value"));
    assert!(
        !store.query(&query()).unwrap()[0]
            .redacted_attribute_keys()
            .is_empty()
    );
}
#[test]
fn explicit_export_roundtrip_receives_only_redacted_candidate_bound_events() {
    let dir = TestDir::new("export-ok");
    let store = store(&dir);
    let mut item = event("exported", 1, 1, "fail");
    item.add_public_attribute("token", "Bearer secret-value")
        .unwrap();
    store.append(&item).unwrap();
    let mut adapter = MockAdapter::new(AdapterMode::Ok);

    let count = store
        .export_explicit(ExplicitExportRequest {
            query: &query(),
            configured: true,
            consent_granted: true,
            timeout: Duration::from_secs(1),
            adapter: Some(&mut adapter),
        })
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(adapter.export_calls, 1);
    let payload = serde_json::to_string(&adapter.exported).unwrap();
    assert!(!payload.contains("secret-value"));
    assert!(payload.contains("redacted_attribute_keys"));
}

#[test]
fn absent_disabled_denied_unavailable_wrong_candidate_and_unredacted_adapters_fail_before_effect() {
    let dir = TestDir::new("export-gates");
    let store = store(&dir);
    store.append(&event("event", 1, 1, "fail")).unwrap();
    let timeout = Duration::from_secs(1);

    assert!(
        store
            .export_explicit(ExplicitExportRequest {
                query: &query(),
                configured: false,
                consent_granted: true,
                timeout,
                adapter: None,
            })
            .unwrap_err()
            .contains("disabled")
    );
    assert!(
        store
            .export_explicit(ExplicitExportRequest {
                query: &query(),
                configured: true,
                consent_granted: false,
                timeout,
                adapter: None,
            })
            .unwrap_err()
            .contains("consent")
    );
    assert!(
        store
            .export_explicit(ExplicitExportRequest {
                query: &query(),
                configured: true,
                consent_granted: true,
                timeout,
                adapter: None,
            })
            .unwrap_err()
            .contains("absent")
    );

    let mut unavailable = MockAdapter::new(AdapterMode::Ok);
    unavailable.available = false;
    assert!(
        store
            .export_explicit(ExplicitExportRequest {
                query: &query(),
                configured: true,
                consent_granted: true,
                timeout,
                adapter: Some(&mut unavailable),
            })
            .is_err()
    );
    assert_eq!(unavailable.export_calls, 0);

    let mut wrong_candidate = MockAdapter::new(AdapterMode::Ok);
    wrong_candidate.candidate_id = "cand-2".to_owned();
    assert!(
        store
            .export_explicit(ExplicitExportRequest {
                query: &query(),
                configured: true,
                consent_granted: true,
                timeout,
                adapter: Some(&mut wrong_candidate),
            })
            .unwrap_err()
            .contains("wrong-candidate")
    );
    assert_eq!(wrong_candidate.export_calls, 0);

    let mut unredacted_contract = MockAdapter::new(AdapterMode::Ok);
    unredacted_contract.redacted_only = false;
    assert!(
        store
            .export_explicit(ExplicitExportRequest {
                query: &query(),
                configured: true,
                consent_granted: true,
                timeout,
                adapter: Some(&mut unredacted_contract),
            })
            .unwrap_err()
            .contains("redaction-contract")
    );
    assert_eq!(unredacted_contract.export_calls, 0);
}

#[test]
fn outage_delay_duplicate_and_partial_ack_lower_only_export_proof() {
    for (label, mode, timeout, expected) in [
        (
            "outage",
            AdapterMode::Outage,
            Duration::from_secs(1),
            "outage",
        ),
        (
            "delay",
            AdapterMode::Delay,
            Duration::from_millis(1),
            "timeout",
        ),
        (
            "duplicate",
            AdapterMode::Duplicate,
            Duration::from_secs(1),
            "duplicate",
        ),
        (
            "partial",
            AdapterMode::Partial,
            Duration::from_secs(1),
            "partial",
        ),
        (
            "reconcile-partial",
            AdapterMode::ReconcilePartial,
            Duration::from_secs(1),
            "partial",
        ),
    ] {
        let dir = TestDir::new(label);
        let store = store(&dir);
        store.append(&event("event", 1, 1, "fail")).unwrap();
        let mut adapter = MockAdapter::new(mode);
        let error = store
            .export_explicit(ExplicitExportRequest {
                query: &query(),
                configured: true,
                consent_granted: true,
                timeout,
                adapter: Some(&mut adapter),
            })
            .unwrap_err();
        assert!(error.contains(expected), "{label}: {error}");
        assert_eq!(
            store.query(&query()).unwrap().len(),
            1,
            "local path survives {label}"
        );
        assert_eq!(
            store.explain(&query(), "event").unwrap().classification(),
            "observed-cause"
        );
    }
}
