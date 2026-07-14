#[test]
fn missing_extra_and_write_capable_authority_are_reported_exactly() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    fs::remove_file(host.installed.join(".codex/agents/security-reviewer.toml")).unwrap();
    host.write(
        &host.cache,
        ".codex/agents/extra-reviewer.toml",
        descriptor("extra-reviewer", "read-only"),
    );
    host.write(
        &host.project,
        ".codex/agents/repo-recon.toml",
        descriptor("repo-recon", "workspace-write"),
    );
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = exact_reader(&source, &host);

    assert!(session.verify(&mut reader).is_err());
    let findings = reader.report().findings().to_vec();
    assert!(findings.iter().any(|finding| {
        finding.layer() == AgentAuthorityLayer::Installed
            && finding.kind() == SupportedAgentAuthorityFindingKind::MissingCanonical
            && finding.authority_name() == "security-reviewer"
    }));
    assert!(findings.iter().any(|finding| {
        finding.layer() == AgentAuthorityLayer::Cache
            && finding.kind() == SupportedAgentAuthorityFindingKind::ExtraAuthority
            && finding.authority_name() == "extra-reviewer"
    }));
    assert!(findings.iter().any(|finding| {
        finding.layer() == AgentAuthorityLayer::Discovery
            && finding.kind() == SupportedAgentAuthorityFindingKind::WriteCapableSandbox
            && finding.authority_name() == "repo-recon"
    }));
}

#[test]
fn stale_same_version_installed_bytes_cannot_substitute_for_current_source() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    host.write(
        &host.installed,
        ".codex/agents/research-verifier.toml",
        descriptor("research-verifier", "workspace-write"),
    );
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = exact_reader(&source, &host);

    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::SandboxPolicyRejected
    );
    assert!(reader.report().findings().iter().any(|finding| {
        finding.kind() == SupportedAgentAuthorityFindingKind::WriteCapableSandbox
            && finding.authority_name() == "research-verifier"
    }));
}

#[test]
fn same_version_manifest_substitution_is_reported_as_plugin_identity_drift() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    let manifest_path = host.installed.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["description"] = serde_json::json!("same version, different bytes");
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = exact_reader(&source, &host);

    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::IdentityMismatch
    );
    assert!(reader.report().findings().iter().any(|finding| {
        finding.layer() == AgentAuthorityLayer::Installed
            && finding.kind() == SupportedAgentAuthorityFindingKind::PluginIdentityMismatch
    }));
}

#[test]
fn descriptor_mutation_after_transaction_open_invalidates_the_generation() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    let path = host
        .package
        .join(".codex/agents/product-journey-reviewer.toml");
    let mut reader = exact_reader(&source, &host);
    reader.set_after_transaction_open_hook(move || {
        fs::write(
            path,
            descriptor("product-journey-reviewer", "workspace-write"),
        )
        .unwrap();
    });
    let session = AgentDiscoverySession::bind(source).unwrap();

    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationUnavailable
    );
    assert_eq!(reader.report().failure_code(), Some("observation-changed"));
}

#[test]
fn ancestor_replacement_after_transaction_open_is_rejected() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    let package = host.package.clone();
    let displaced = host.root.join("package-displaced");
    let mut reader = exact_reader(&source, &host);
    reader.set_after_transaction_open_hook(move || {
        fs::rename(&package, &displaced).unwrap();
        fs::create_dir_all(&package).unwrap();
    });
    let session = AgentDiscoverySession::bind(source).unwrap();

    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::ObservationUnavailable
    );
}

#[cfg(unix)]
#[test]
fn symlink_and_hardlink_host_entries_fail_closed_without_writes() {
    use std::os::unix::fs::symlink;

    for hardlink in [false, true] {
        let repo = TempRepo::canonical();
        let source = repo.capture();
        let host = SupportedHostFixture::exact(&source);
        let target = host.global.join(".codex/agents/unrelated-observer.toml");
        let outside = host.root.join("outside.toml");
        fs::write(&outside, descriptor("unrelated-observer", "read-only")).unwrap();
        if hardlink {
            fs::hard_link(&outside, &target).unwrap();
        } else {
            symlink(&outside, &target).unwrap();
        }
        let before = tree_snapshot(&host.root);
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
        assert_eq!(tree_snapshot(&host.root), before);
    }
}

#[cfg(unix)]
#[test]
fn fifo_host_entry_is_nonblocking_and_refused() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    let path = host.global.join(".codex/agents/pipe.toml");
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let before = tree_snapshot(&host.root);
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
    assert_eq!(tree_snapshot(&host.root), before);
}
