use serde_json::json;

use crate::distribution::{Capability, DistributionErrorId, Layer, LayerVerdict, verify};
use crate::distribution_fixture::{Fixture, digest};

#[test]
fn stale_mixed_hidden_and_payload_mismatch_never_promote() {
    let stale = Fixture::complete("stale-version");
    stale.mutate_envelope(Layer::MarketplaceCatalog, |value| {
        value["version"] = json!("0.0.10")
    });
    let report = verify(&stale.root, &stale.bytes()).expect("stale report");
    assert!(!report.accepted());
    assert_eq!(
        report.layer(Layer::MarketplaceCatalog).verdict(),
        LayerVerdict::StaleIdentity
    );

    let mixed = Fixture::complete("mixed-candidate");
    mixed.mutate_envelope(Layer::Discovery, |value| {
        value["context_id"] =
            json!("sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
    });
    let report = verify(&mixed.root, &mixed.bytes()).expect("mixed report");
    assert_eq!(
        report.layer(Layer::Discovery).verdict(),
        LayerVerdict::MixedCandidate
    );

    let hidden = Fixture::complete("registered-hidden");
    hidden.mutate_envelope(Layer::AppRegistryUi, |value| {
        value["exposure"] = json!("denied")
    });
    let report = verify(&hidden.root, &hidden.bytes()).expect("hidden report");
    assert_eq!(
        report.layer(Layer::AppRegistryUi).verdict(),
        LayerVerdict::Contradicted
    );

    let mismatch = Fixture::complete("payload-mismatch");
    std::fs::write(
        mismatch.root.join("payload/cache-bytes.bin"),
        b"different cache bytes\n",
    )
    .unwrap();
    let report = verify(&mismatch.root, &mismatch.bytes()).expect("payload mismatch report");
    assert_eq!(
        report.layer(Layer::CacheBytes).verdict(),
        LayerVerdict::PayloadMismatch
    );

    let joined = Fixture::complete("joined-stale-bytes");
    let installed = b"internally valid but wrong installed bytes\n";
    std::fs::write(joined.root.join("payload/installed-bytes.bin"), installed).unwrap();
    let installed_sha = digest(installed);
    joined.mutate_envelope(Layer::InstalledBytes, |value| {
        value["artifact_sha256"] = json!(installed_sha)
    });
    let report = verify(&joined.root, &joined.bytes()).expect("join mismatch report");
    assert_eq!(
        report.layer(Layer::InstalledBytes).verdict(),
        LayerVerdict::StaleIdentity
    );
}

#[test]
fn app_registry_and_plugins_ui_are_distinct_non_promoting_observations() {
    let registry = Fixture::complete("app-registry-hidden");
    registry.mutate_envelope(Layer::AppRegistry, |value| {
        value["exposure"] = json!("denied")
    });
    let report = verify(&registry.root, &registry.bytes()).expect("registry report");
    assert_eq!(
        report.layer(Layer::AppRegistry).verdict(),
        LayerVerdict::Contradicted
    );
    assert_eq!(
        report.layer(Layer::AppRegistryUi).verdict(),
        LayerVerdict::ObservedUnjoined
    );

    let ui = Fixture::complete("plugins-ui-hidden");
    ui.mutate_envelope(Layer::AppRegistryUi, |value| {
        value["exposure"] = json!("denied")
    });
    let report = verify(&ui.root, &ui.bytes()).expect("UI report");
    assert_eq!(
        report.layer(Layer::AppRegistry).verdict(),
        LayerVerdict::Verified
    );
    assert_eq!(
        report.layer(Layer::AppRegistryUi).verdict(),
        LayerVerdict::Contradicted
    );
    assert!(!report.accepted());
}

#[test]
fn duplicate_unknown_and_malformed_specs_fail_with_stable_non_echo_errors() {
    let fixture = Fixture::complete("spec-errors");
    let mut duplicate = fixture.request.clone();
    let first = duplicate["layers"][0].clone();
    duplicate["layers"].as_array_mut().unwrap().push(first);
    assert_error(
        verify(&fixture.root, &serde_json::to_vec(&duplicate).unwrap()).unwrap_err(),
        DistributionErrorId::DuplicateLayer,
    );

    let mut duplicate_capability = fixture.request.clone();
    let first = duplicate_capability["host"]["capabilities"][0].clone();
    duplicate_capability["host"]["capabilities"]
        .as_array_mut()
        .unwrap()
        .push(first);
    assert_error(
        verify(
            &fixture.root,
            &serde_json::to_vec(&duplicate_capability).unwrap(),
        )
        .unwrap_err(),
        DistributionErrorId::DuplicateCapability,
    );

    let mut unknown = fixture.request.clone();
    unknown["unknown_root_authority"] = json!(true);
    assert_error(
        verify(&fixture.root, &serde_json::to_vec(&unknown).unwrap()).unwrap_err(),
        DistributionErrorId::InvalidSpec,
    );

    let mut unknown_layer = fixture.request.clone();
    unknown_layer["layers"][0]["layer"] = json!("unknown-layer-canary");
    assert_error(
        verify(&fixture.root, &serde_json::to_vec(&unknown_layer).unwrap()).unwrap_err(),
        DistributionErrorId::InvalidSpec,
    );

    let duplicate_key = br#"{
        "schema":"harness-ultragoal.distribution-request.v1",
        "context_id":"one",
        "context_id":"two"
    }"#;
    assert_error(
        verify(&fixture.root, duplicate_key).unwrap_err(),
        DistributionErrorId::InvalidJson,
    );

    let mut unavailable_observed = fixture.request.clone();
    unavailable_observed["host"]["capabilities"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["capability"] == Capability::Runtime.as_str())
        .unwrap()["state"] = json!("unavailable");
    assert_error(
        verify(
            &fixture.root,
            &serde_json::to_vec(&unavailable_observed).unwrap(),
        )
        .unwrap_err(),
        DistributionErrorId::CapabilityMismatch,
    );
}

fn assert_error(error: crate::distribution::DistributionError, id: DistributionErrorId) {
    assert_eq!(error.id(), id);
    assert!(error.to_string().len() <= 64);
    assert!(!error.to_string().contains("canary"));
}
