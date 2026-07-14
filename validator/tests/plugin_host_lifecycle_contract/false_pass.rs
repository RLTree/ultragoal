use crate::host_fixture::{Fixture, Reader, handoff_external_effect, lifecycle, request};
use crate::host_lifecycle::{HostLifecycleErrorId, HostLifecyclePhase};
use crate::plugin_product::lifecycle::{LifecycleIntent, LifecycleState};

#[test]
fn package_marketplace_and_receipt_shaped_values_cannot_substitute_for_host_observations() {
    let fixture = Fixture::new("lower-layer-false-pass");
    let bundle = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    let plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let (mut session, _) = fixture.session(&bundle, plan);
    session.apply_confined(&empty).unwrap();
    handoff_external_effect(&mut session).unwrap();
    for canary in [
        bundle.snapshot.archive().to_vec(),
        b"{\"status\":\"installed\"}".to_vec(),
        b"documentation says visible".to_vec(),
    ] {
        let mut reader = Reader {
            marketplace: Some(
                crate::host_fixture::marketplace_plan(&bundle)
                    .replacement()
                    .to_vec(),
            ),
            cache: Some(canary.clone()),
            registry: Some(canary),
            ..Reader::default()
        };
        reader.provenance_sha256 = session.binding().binding_sha256().to_owned();
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert!(matches!(
            error.id(),
            HostLifecycleErrorId::ObservationConflict
                | HostLifecycleErrorId::ObservationUnavailable
        ));
    }
}

#[test]
fn source_local_report_never_has_claim_effect_even_with_a_complete_identity_chain() {
    let fixture = Fixture::new("no-claim-report");
    let bundle = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    let plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let (mut session, host) = fixture.session(&bundle, plan);
    session.apply_confined(&empty).unwrap();
    handoff_external_effect(&mut session).unwrap();
    let mut reader = Reader::complete(&bundle, &host, session.binding());
    let report = session.capture_and_verify(&mut reader).unwrap();
    assert!(report.identity_chain_sha256().is_some());
    assert_eq!(
        report.phase(),
        HostLifecyclePhase::AwaitingSupportedHostObservation
    );
    assert!(!report.has_claim_effect());
    let serialized = serde_json::to_value(&report).unwrap();
    assert_eq!(serialized["claim_effect"], false);
}

#[test]
fn coordinator_source_has_no_live_host_executor_or_claim_authority() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/plugin_product/host_lifecycle");
    let mut source = String::new();
    for entry in std::fs::read_dir(&root).unwrap() {
        source.push_str(&std::fs::read_to_string(entry.unwrap().path()).unwrap());
    }
    for forbidden in [
        "execute_authorized(",
        "std::process::Command",
        "claim_decision",
        "proven_live",
        "HostAuthorization::new",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden authority: {forbidden}"
        );
    }
    let session = std::fs::read_to_string(root.join("session.rs")).unwrap();
    let module = std::fs::read_to_string(root.join("mod.rs")).unwrap();
    assert!(session.contains("pub fn capture_and_verify("));
    assert!(!session.contains("pub fn capture_observations("));
    assert!(!session.contains("pub fn verify_observations("));
    assert!(!session.contains("pub fn session_issuance_sha256("));
    assert!(!session.contains("pub fn host_scope_sha256("));
    assert!(!module.contains("capture_host_observations"));
    assert!(!module.contains("HostObservationExpectations"));

    let public_start = session.find("pub fn capture_and_verify(").unwrap();
    let transaction_start = session.find("fn capture_and_verify_transaction(").unwrap();
    let public_capture = &session[public_start..transaction_start];
    let preflight = public_capture
        .find("self.require_observation_preflight()?")
        .unwrap();
    let request = public_capture
        .find("HostObservationTransactionRequest::issue(")
        .unwrap();
    let adapter = public_capture.find("reader.with_transaction(").unwrap();
    assert!(preflight < request && request < adapter);
}
