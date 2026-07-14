#[test]
fn wrong_project_candidate_session_nonce_or_fresh_session_identity_is_rejected() {
    for field in [
        "project_root_sha256",
        "candidate_id",
        "session_id",
        "session_issuance_sha256",
        "observation_nonce_sha256",
    ] {
        assert_eq!(
            verify_with(move |transaction| {
                transaction.mutate_catalog(AgentAuthorityLayer::Discovery, |catalog| {
                    catalog[field] = json!(
                        "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                    );
                });
            }),
            AgentDiscoveryErrorId::IdentityMismatch
        );
    }
    assert_eq!(
        verify_with(|transaction| {
            transaction.mutate_catalog(AgentAuthorityLayer::Discovery, |catalog| {
                catalog["new_session"] = json!(false);
            });
        }),
        AgentDiscoveryErrorId::IdentityMismatch
    );
}

#[test]
fn transaction_level_identity_mismatch_stops_before_catalog_reads() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(|transaction| transaction.candidate = "sha256:bad".to_owned());
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationChanged
    );
    assert_eq!(reader.transaction().reads, 0);
}

#[test]
fn authority_roots_generation_and_transaction_provenance_are_not_interchangeable() {
    assert_eq!(
        verify_with(|transaction| {
            let package: serde_json::Value =
                serde_json::from_slice(&transaction.catalogs[&AgentAuthorityLayer::Package])
                    .unwrap();
            let package_root = package["authority_root_sha256"].clone();
            transaction.mutate_catalog(AgentAuthorityLayer::Installed, |catalog| {
                catalog["authority_root_sha256"] = package_root;
            });
        }),
        AgentDiscoveryErrorId::IdentityMismatch
    );
    assert_eq!(
        verify_with(|transaction| {
            transaction.mutate_catalog(AgentAuthorityLayer::Cache, |catalog| {
                catalog["authority_generation_sha256"] = json!(
                    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                );
            });
        }),
        AgentDiscoveryErrorId::IdentityMismatch
    );
    assert_eq!(
        verify_with(|transaction| {
            transaction.mutate_catalog(AgentAuthorityLayer::Discovery, |catalog| {
                catalog["transaction_provenance_sha256"] = json!(
                    "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
                );
            });
        }),
        AgentDiscoveryErrorId::IdentityMismatch
    );
}
