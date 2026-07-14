#[cfg(unix)]
#[test]
fn socket_and_unreadable_host_entries_are_refused_without_blocking() {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;

    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    let socket_path = host.global.join(".codex/agents/socket.toml");
    let _listener = UnixListener::bind(&socket_path).unwrap();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = SupportedHostAgentAuthorityReader::open(source.clone(), host.roots()).unwrap();
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationUnavailable
    );
    assert_eq!(
        reader.report().failure_code(),
        Some("unsafe-filesystem-entry")
    );
    drop(_listener);
    fs::remove_file(&socket_path).unwrap();

    let unreadable = host.global.join(".codex/agents/unreadable.toml");
    fs::write(&unreadable, descriptor("unreadable", "read-only")).unwrap();
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0)).unwrap();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = SupportedHostAgentAuthorityReader::open(source, host.roots()).unwrap();
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationUnavailable
    );
    assert_eq!(
        reader.report().failure_code(),
        Some("unsafe-filesystem-entry")
    );
}

#[cfg(unix)]
#[test]
fn agent_root_symlink_path_escape_is_rejected_before_transaction_creation() {
    use std::os::unix::fs::symlink;

    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    let outside = host.root.join("outside-agents");
    fs::create_dir(&outside).unwrap();
    fs::remove_dir(host.global.join(".codex/agents")).unwrap();
    symlink(&outside, host.global.join(".codex/agents")).unwrap();

    let error = match SupportedHostAgentAuthorityReader::open(source, host.roots()) {
        Ok(_) => panic!("symlinked agent root was accepted"),
        Err(error) => error,
    };
    assert_eq!(error.id(), AgentDiscoveryErrorId::UnsafeFilesystemEntry);
}

#[cfg(unix)]
#[test]
fn non_utf8_and_oversize_host_entries_are_refused_with_stable_diagnostics() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    fs::write(
        host.global.join(".codex/agents/non-utf8.toml"),
        [b'n', b'a', b'm', b'e', b' ', b'=', b' ', b'\"', 0xff, b'\"'],
    )
    .unwrap();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = SupportedHostAgentAuthorityReader::open(source.clone(), host.roots()).unwrap();
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationUnavailable
    );
    assert_eq!(
        reader.report().failure_code(),
        Some("unsafe-filesystem-entry")
    );
    fs::remove_file(host.global.join(".codex/agents/non-utf8.toml")).unwrap();
    fs::write(
        host.global.join(".codex/agents/oversize.toml"),
        vec![b'x'; 64 * 1024 + 1],
    )
    .unwrap();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = SupportedHostAgentAuthorityReader::open(source, host.roots()).unwrap();
    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationUnavailable
    );
    assert_eq!(reader.report().failure_code(), Some("input-too-large"));
}

#[test]
fn duplicate_descriptor_keys_and_source_only_receipts_do_not_become_host_evidence() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    let duplicate = format!(
        "{}sandbox_mode = \"read-only\"\n",
        descriptor("security-reviewer", "read-only")
    );
    host.write(
        &host.project,
        ".codex/agents/security-reviewer.toml",
        duplicate,
    );
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = exact_reader(&source, &host);

    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationConflict
    );
    assert!(reader.report().findings().iter().any(|finding| {
        finding.kind() == SupportedAgentAuthorityFindingKind::InvalidDescriptor
            && finding.authority_name() == "security-reviewer"
    }));
}

#[test]
fn supported_transaction_remains_single_use_through_the_verifier_session() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = exact_reader(&source, &host);
    session.verify(&mut reader).unwrap();

    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::SessionReplay
    );
}

#[test]
fn every_canonical_role_is_observed_in_each_required_non_global_layer() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = exact_reader(&source, &host);
    session.verify(&mut reader).unwrap();
    let report = reader.report();

    for layer in [
        AgentAuthorityLayer::Package,
        AgentAuthorityLayer::Installed,
        AgentAuthorityLayer::Cache,
        AgentAuthorityLayer::Discovery,
    ] {
        for role in canonical_names() {
            assert!(report.observations().iter().any(|observation| {
                observation.layer() == layer
                    && observation.authority_name() == role
                    && observation.sandbox_mode() == Some("read-only")
            }));
        }
    }
}

#[test]
fn source_local_reader_has_no_write_or_fresh_session_claim_surface() {
    let reader_source =
        include_str!("../../../src/plugin_product/agent_discovery/supported/mod.rs");
    let module_wiring = include_str!("../../../src/plugin_product/mod.rs");

    assert!(!reader_source.contains("new_session: true"));
    assert!(!reader_source.contains("std::fs::write"));
    assert!(!reader_source.contains("OpenOptions"));
    assert!(!reader_source.contains("File::create"));
    assert!(module_wiring.contains("pub(crate) mod agent_discovery;"));
    assert!(!module_wiring.contains("pub mod agent_discovery;"));
}
