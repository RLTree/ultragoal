use crate::agent_discovery::{AgentAuthorityLayer, AgentDiscoveryErrorId, AgentDiscoverySession};
use crate::support::{FixtureReader, TempRepo, descriptor, digest};
use serde_json::json;

const LIVE_LEGACY_AGENTS: [(&str, &str, &str); 7] = [
    (
        "harness_contract_claim_falsifier",
        "custom-agents/harness-contract-claim-falsifier.toml",
        include_str!("../../../custom-agents/harness-contract-claim-falsifier.toml"),
    ),
    (
        "harness_material_review_scope_gatekeeper",
        "custom-agents/harness-material-review-scope-gatekeeper.toml",
        include_str!("../../../custom-agents/harness-material-review-scope-gatekeeper.toml"),
    ),
    (
        "harness_orchestration_recovery_falsifier",
        "custom-agents/harness-orchestration-recovery-falsifier.toml",
        include_str!("../../../custom-agents/harness-orchestration-recovery-falsifier.toml"),
    ),
    (
        "harness_product_simplicity_falsifier",
        "custom-agents/harness-product-simplicity-falsifier.toml",
        include_str!("../../../custom-agents/harness-product-simplicity-falsifier.toml"),
    ),
    (
        "harness_repo_initializer",
        "custom-agents/harness-repo-initializer.toml",
        include_str!("../../../custom-agents/harness-repo-initializer.toml"),
    ),
    (
        "harness_retrofit_planner",
        "custom-agents/harness-retrofit-planner.toml",
        include_str!("../../../custom-agents/harness-retrofit-planner.toml"),
    ),
    (
        "harness_security_trust_boundary_falsifier",
        "custom-agents/harness-security-trust-boundary-falsifier.toml",
        include_str!("../../../custom-agents/harness-security-trust-boundary-falsifier.toml"),
    ),
];

fn verify_with(
    configure: impl FnOnce(&mut crate::support::FixtureTransaction) + 'static,
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
fn active_legacy_and_normalized_global_collisions_are_distinct_blockers() {
    assert_eq!(
        verify_with(|transaction| {
            transaction.add_global_agent("harness-contract-claim-falsifier", Some("read-only"));
        }),
        AgentDiscoveryErrorId::LegacyAuthorityActive
    );
    assert_eq!(
        verify_with(|transaction| {
            transaction.add_global_agent("claim_falsifier", Some("read-only"));
        }),
        AgentDiscoveryErrorId::CollidingAuthorityActive
    );
}

#[test]
fn every_live_underscore_legacy_identifier_blocks_independently_with_exact_bytes() {
    for (name, path, bytes) in LIVE_LEGACY_AGENTS {
        let label = name;
        let name = name.to_owned();
        let path = path.to_owned();
        let bytes = bytes.to_owned();
        assert_eq!(
            verify_with(move |transaction| {
                transaction.add_global_agent_bytes(&name, &path, bytes);
            }),
            AgentDiscoveryErrorId::LegacyAuthorityActive,
            "legacy authority unexpectedly escaped classification: {label}"
        );
    }
}

#[test]
fn removing_additional_hyphenated_roles_cannot_hide_remaining_underscore_legacy_authority() {
    assert_eq!(
        verify_with(|transaction| {
            transaction.add_global_agent("harness-cost-benefit-reviewer", None);
            transaction.add_global_agent("harness-log-scout", None);
            for (name, path, bytes) in LIVE_LEGACY_AGENTS {
                transaction.add_global_agent_bytes(name, path, bytes);
            }
            transaction.remove_global_agent("harness-cost-benefit-reviewer");
            transaction.remove_global_agent("harness-log-scout");
        }),
        AgentDiscoveryErrorId::LegacyAuthorityActive
    );
}

#[test]
fn omitted_sandbox_and_write_capable_effect_observations_block_eligibility() {
    assert_eq!(
        verify_with(|transaction| {
            transaction.add_global_agent("unrelated-observer", None);
        }),
        AgentDiscoveryErrorId::ObservationConflict
    );
    assert_eq!(
        verify_with(|transaction| transaction.forge_effect = true),
        AgentDiscoveryErrorId::SandboxPolicyRejected
    );
}

#[test]
fn write_capable_unrelated_global_authority_cannot_produce_eligibility() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(|transaction| {
        transaction.add_global_agent("unrelated-observer", Some("workspace-write"));
    });

    let result = session.verify(&mut reader);

    assert_eq!(
        result.unwrap_err().id(),
        AgentDiscoveryErrorId::SandboxPolicyRejected
    );
    assert_eq!(reader.transaction().probes, 0);
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
