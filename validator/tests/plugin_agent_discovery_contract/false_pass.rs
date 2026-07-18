use super::super::{
    AgentAuthorityLayer, AgentDiscoveryErrorId, AgentDiscoverySession,
    HostAgentAuthorityTransactionError,
};
use super::authority_fixtures::{FixtureReader, TempRepo};

#[test]
fn source_catalog_without_host_transaction_cannot_produce_eligibility() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.unavailable = Some(HostAgentAuthorityTransactionError::Unsupported);
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationUnavailable
    );
}

#[test]
fn catalog_names_and_descriptor_digests_cannot_replace_descriptor_bytes() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(|transaction| {
        transaction.mutate_catalog(AgentAuthorityLayer::Installed, |catalog| {
            catalog["agents"][0]["descriptor_toml"] = serde_json::json!("");
        });
    });
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationConflict
    );
}

#[test]
fn source_version_or_receipt_shaped_manifest_digest_cannot_mask_installed_bytes() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(|transaction| {
        transaction.mutate_catalog(AgentAuthorityLayer::Installed, |catalog| {
            let mut manifest: serde_json::Value =
                serde_json::from_str(catalog["plugin_manifest_json"].as_str().unwrap()).unwrap();
            manifest["description"] = serde_json::json!("stale installed bytes");
            catalog["plugin_manifest_json"] =
                serde_json::json!(serde_json::to_string(&manifest).unwrap());
            // Keep the source digest to model a proof-shaped receipt assertion.
        });
    });
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::IdentityMismatch
    );
}

#[test]
fn fixture_eligibility_report_has_no_claim_or_adoption_effect() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    let report = session.verify(&mut reader).unwrap();
    assert!(report.route_eligible());
    assert!(!report.has_claim_effect());
}

#[test]
fn currently_observed_legacy_shape_blocks_even_when_version_strings_match() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(|transaction| {
        transaction.add_global_agent(
            "harness-security-trust-boundary-falsifier",
            Some("read-only"),
        );
    });
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::LegacyAuthorityActive
    );
}
