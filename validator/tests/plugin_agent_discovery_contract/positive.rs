use crate::agent_discovery::{AgentAuthorityLayer, AgentDiscoverySession, SourceAgentCatalog};
use crate::authority_fixtures::{
    CANDIDATE, FixtureReader, SESSION, TempRepo, canonical_names, descriptor, tree_snapshot,
};
use std::fs;

#[test]
fn exact_six_roles_and_fresh_bound_host_transaction_are_eligible_without_claim_effect() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    let report = session.verify(&mut reader).unwrap();

    assert!(report.route_eligible());
    assert!(report.new_session_observed());
    assert!(!report.has_claim_effect());
    assert_eq!(report.layers().len(), AgentAuthorityLayer::ALL.len());
    assert_eq!(reader.transaction().reads, 10);
    assert_eq!(reader.transaction().probes, 6);
    for layer in report.layers() {
        if layer.layer() == AgentAuthorityLayer::Global {
            assert!(layer.canonical_agents().is_empty());
        } else {
            assert_eq!(layer.canonical_agents().len(), 6);
        }
    }
}

#[test]
fn source_capture_and_revalidation_are_recursively_zero_write() {
    let repo = TempRepo::canonical();
    let before = tree_snapshot(&repo.root);
    let source = repo.capture();
    source.revalidate().unwrap();
    let after = tree_snapshot(&repo.root);
    assert_eq!(before, after);
}

#[test]
fn exact_name_validation_is_independent_of_directory_enumeration_order() {
    let repo = TempRepo::canonical();
    for name in canonical_names() {
        fs::remove_file(repo.root.join(format!(".codex/agents/{name}.toml"))).unwrap();
    }
    for name in canonical_names().into_iter().rev() {
        fs::write(
            repo.root.join(format!(".codex/agents/{name}.toml")),
            descriptor(name, "read-only"),
        )
        .unwrap();
    }
    let source = SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION).unwrap();
    assert_eq!(source.canonical_agents().len(), 6);
}
