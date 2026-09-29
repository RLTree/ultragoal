use super::super::{AgentAuthorityLayer, AgentDiscoveryErrorId, AgentDiscoverySession};
use super::authority_fixtures::{FixtureReader, TempRepo, descriptor};

#[test]
fn two_capture_catalog_drift_is_rejected_before_effect_probes() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(|transaction| {
        let mut second = transaction.catalogs.clone();
        let bytes = second.get(&AgentAuthorityLayer::Cache).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        value["new_session"] = serde_json::json!(false);
        second.insert(
            AgentAuthorityLayer::Cache,
            serde_json::to_vec(&value).unwrap(),
        );
        transaction.second_catalogs = Some(second);
    });
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationChanged
    );
    assert_eq!(reader.transaction().probes, 0);
}

#[test]
fn generation_drift_fails_before_any_host_read() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(|transaction| transaction.current_generation += 1);
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationChanged
    );
    assert_eq!(reader.transaction().reads, 0);
}

#[test]
fn source_descriptor_drift_during_transaction_is_rejected() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let path = repo.root.join(".codex/agents/repo-recon.toml");
    let drifted = descriptor("repo-recon", "read-only")
        .replace("Inspect bounded evidence", "Review bounded evidence")
        .into_bytes();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(move |transaction| {
        transaction.mutate_path_on_read = Some((1, path, drifted));
    });
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationChanged
    );
}

#[test]
fn completed_or_rejected_sessions_cannot_be_replayed() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut first = FixtureReader::exact(&source);
    assert!(session.verify(&mut first).is_ok());
    let mut replay = FixtureReader::exact(&source);
    assert_eq!(
        session.verify(&mut replay).unwrap_err().id(),
        AgentDiscoveryErrorId::SessionReplay
    );
    assert_eq!(replay.calls, 0);

    let rejected = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut bad = FixtureReader::exact(&source);
    bad.configure(|transaction| transaction.current_generation += 1);
    assert!(rejected.verify(&mut bad).is_err());
    let mut retry = FixtureReader::exact(&source);
    assert_eq!(
        rejected.verify(&mut retry).unwrap_err().id(),
        AgentDiscoveryErrorId::SessionStateRejected
    );
    assert_eq!(retry.calls, 0);
}

#[test]
fn sibling_sessions_with_identical_public_ids_have_distinct_private_issuance() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let first = AgentDiscoverySession::bind(source.clone()).unwrap();
    let second = AgentDiscoverySession::bind(source.clone()).unwrap();
    assert_ne!(first.binding_sha256(), second.binding_sha256());
}

#[test]
fn canonical_ancestor_replacement_invalidates_the_bound_source_before_host_contact() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let original = repo.root.join(".codex");
    let moved = repo.root.join(".codex-old");
    std::fs::rename(&original, &moved).unwrap();
    std::fs::create_dir_all(&original).unwrap();
    std::fs::rename(moved.join("agents"), original.join("agents")).unwrap();
    let mut reader = FixtureReader::exact(&source);
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationChanged
    );
    assert_eq!(reader.calls, 0);
}

#[cfg(unix)]
#[test]
fn agent_root_swap_to_outside_bytes_and_restore_is_rejected_after_anchored_reads() {
    use std::os::unix::fs::symlink;

    let repo = TempRepo::canonical();
    let live = repo.root.join(".codex/agents");
    let retained = repo.root.join(".codex/agents-retained");
    let outside = repo.root.join("outside-agents");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(
        outside.join("repo-recon.toml"),
        b"SECRET-OUTSIDE-AUTHORITY-BYTES",
    )
    .unwrap();
    let source = repo.capture();
    let live_before = live.clone();
    let retained_before = retained.clone();
    let outside_before = outside.clone();
    let live_after = live.clone();
    let retained_after = retained.clone();
    let anchored = source.clone();
    let error = source
        .revalidate_with_test_hooks(
            move || {
                std::fs::rename(&live_before, &retained_before).unwrap();
                symlink(&outside_before, &live_before).unwrap();
                assert_eq!(
                    anchored
                        .revalidate_anchored_contents_for_test()
                        .unwrap_err()
                        .id(),
                    AgentDiscoveryErrorId::ObservationChanged
                );
            },
            move || {
                std::fs::remove_file(&live_after).unwrap();
                std::fs::rename(&retained_after, &live_after).unwrap();
            },
        )
        .unwrap_err();
    assert_eq!(error.id(), AgentDiscoveryErrorId::ObservationChanged);
    assert!(
        !source
            .descriptor_bytes("repo-recon")
            .unwrap()
            .windows(b"SECRET-OUTSIDE-AUTHORITY-BYTES".len())
            .any(|window| window == b"SECRET-OUTSIDE-AUTHORITY-BYTES")
    );
}

#[cfg(unix)]
#[test]
fn plugin_root_swap_to_outside_manifest_and_restore_is_rejected_after_anchored_read() {
    use std::os::unix::fs::symlink;

    let repo = TempRepo::canonical();
    let live = repo.root.join(".codex-plugin");
    let retained = repo.root.join(".codex-plugin-retained");
    let outside = repo.root.join("outside-plugin");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(
        outside.join("plugin.json"),
        br#"{"name":"outside-substitution","version":"0.0.11"}"#,
    )
    .unwrap();
    let source = repo.capture();
    let live_before = live.clone();
    let retained_before = retained.clone();
    let outside_before = outside.clone();
    let live_after = live.clone();
    let retained_after = retained.clone();
    let anchored = source.clone();
    let error = source
        .revalidate_with_test_hooks(
            move || {
                std::fs::rename(&live_before, &retained_before).unwrap();
                symlink(&outside_before, &live_before).unwrap();
                assert_eq!(
                    anchored
                        .revalidate_anchored_contents_for_test()
                        .unwrap_err()
                        .id(),
                    AgentDiscoveryErrorId::ObservationChanged
                );
            },
            move || {
                std::fs::remove_file(&live_after).unwrap();
                std::fs::rename(&retained_after, &live_after).unwrap();
            },
        )
        .unwrap_err();
    assert_eq!(error.id(), AgentDiscoveryErrorId::ObservationChanged);
    assert!(
        !std::str::from_utf8(source.plugin_manifest_bytes())
            .unwrap()
            .contains("outside-substitution")
    );
}

#[test]
fn persistent_agent_directory_replacement_is_rejected_through_retained_handles() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let live = repo.root.join(".codex/agents");
    let retained = repo.root.join(".codex/agents-retained");
    std::fs::rename(&live, &retained).unwrap();
    std::fs::create_dir(&live).unwrap();
    assert_eq!(
        source.revalidate().unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationChanged
    );
}
