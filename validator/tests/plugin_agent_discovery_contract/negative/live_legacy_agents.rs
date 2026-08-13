fn verify_with(
    configure: impl FnOnce(&mut super::authority_fixtures::FixtureTransaction) + 'static,
) -> AgentDiscoveryErrorId {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(configure);
    session.verify(&mut reader).unwrap_err().id()
}

#[test]
fn missing_extra_and_duplicate_canonical_authority_fail_closed() {
    assert_eq!(
        verify_with(|transaction| {
            transaction.mutate_catalog(AgentAuthorityLayer::Installed, |catalog| {
                catalog["agents"].as_array_mut().unwrap().pop();
            });
        }),
        AgentDiscoveryErrorId::ObservationConflict
    );
    assert_eq!(
        verify_with(|transaction| {
            transaction.mutate_catalog(AgentAuthorityLayer::Package, |catalog| {
                let row = catalog["agents"][0].clone();
                catalog["agents"].as_array_mut().unwrap().push(row);
            });
        }),
        AgentDiscoveryErrorId::ObservationConflict
    );
    assert_eq!(
        verify_with(|transaction| {
            transaction.mutate_catalog(AgentAuthorityLayer::Cache, |catalog| {
                let text = descriptor("extra-reviewer", "read-only");
                catalog["agents"].as_array_mut().unwrap().push(json!({
                    "name": "extra-reviewer",
                    "manifest_path": ".codex/agents/extra-reviewer.toml",
                    "descriptor_sha256": digest(text.as_bytes()),
                    "descriptor_toml": text,
                    "file_kind": "regular",
                    "link_count": 1
                }));
            });
        }),
        AgentDiscoveryErrorId::ObservationConflict
    );
}

#[test]
fn canonical_and_closed_wrapper_global_collisions_are_blockers() {
    assert_eq!(
        verify_with(|transaction| {
            transaction.add_global_agent("claim_falsifier", Some("read-only"));
        }),
        AgentDiscoveryErrorId::CollidingAuthorityActive
    );
    for alias in ["harness", "ultragoal", "harness-ultragoal"] {
        let alias = alias.to_owned();
        assert_eq!(
            verify_with(move |transaction| {
                transaction.add_global_agent(&alias, None);
            }),
            AgentDiscoveryErrorId::CollidingAuthorityActive
        );
    }
}

#[test]
fn unrelated_harness_agents_and_global_sandbox_values_are_collision_only() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(|transaction| {
        transaction.add_global_agent("harness-cost-benefit-reviewer", None);
        transaction.add_global_agent("harness-log-scout", Some("workspace-write"));
    });
    assert!(session.verify(&mut reader).is_ok());
}

#[test]
fn forged_non_global_effect_observation_still_blocks_eligibility() {
    assert_eq!(
        verify_with(|transaction| transaction.forge_effect = true),
        AgentDiscoveryErrorId::SandboxPolicyRejected
    );
}

#[test]
fn write_capable_unrelated_global_authority_cannot_grant_plugin_authority() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(|transaction| {
        transaction.add_global_agent("unrelated-observer", Some("workspace-write"));
    });

    let result = session.verify(&mut reader);

    let report = result.unwrap();
    assert!(report.route_eligible());
    assert!(!report.has_claim_effect());
    assert_eq!(reader.transaction().probes, 6);
}

#[test]
fn package_install_cache_and_discovery_bind_exact_current_bytes() {
    for layer in [
        AgentAuthorityLayer::Package,
        AgentAuthorityLayer::Installed,
        AgentAuthorityLayer::Cache,
        AgentAuthorityLayer::Discovery,
    ] {
        assert_eq!(
            verify_with(move |transaction| {
                transaction.mutate_catalog(layer, |catalog| {
                    catalog["plugin_version"] = json!("0.0.10");
                });
            }),
            AgentDiscoveryErrorId::IdentityMismatch
        );
    }
}
