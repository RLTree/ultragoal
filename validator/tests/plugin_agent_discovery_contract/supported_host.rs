use crate::agent_discovery::{
    AgentAuthorityLayer, AgentDiscoveryErrorId, AgentDiscoverySession,
    SupportedAgentAuthorityFindingKind, SupportedHostAgentAuthorityReader,
};
use crate::support::{SupportedHostFixture, TempRepo, canonical_names, descriptor, tree_snapshot};
use std::collections::BTreeSet;
use std::fs;

fn exact_reader(
    source: &crate::agent_discovery::SourceAgentCatalog,
    host: &SupportedHostFixture,
) -> SupportedHostAgentAuthorityReader {
    SupportedHostAgentAuthorityReader::open(source.clone(), host.roots()).unwrap()
}

#[test]
fn supported_reader_executes_the_exact_generation_stable_authority_transaction() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    let before_source = tree_snapshot(&repo.root);
    let before_host = tree_snapshot(&host.root);
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = exact_reader(&source, &host);

    let eligibility = session.verify(&mut reader).unwrap();
    let report = reader.report();

    assert!(!eligibility.route_eligible());
    assert!(!eligibility.new_session_observed());
    assert!(!eligibility.has_claim_effect());
    assert!(report.generation_sha256().is_some());
    assert_eq!(report.capture_count(), 10);
    assert_eq!(report.effect_probe_count(), 6);
    assert_eq!(report.write_operation_count(), 0);
    assert_eq!(report.observations().len(), 24);
    assert!(report.findings().is_empty());
    assert_eq!(
        eligibility
            .layers()
            .iter()
            .map(|layer| layer.authority_root_sha256())
            .collect::<BTreeSet<_>>()
            .len(),
        AgentAuthorityLayer::ALL.len()
    );
    assert_eq!(
        eligibility
            .layers()
            .iter()
            .map(|layer| layer.authority_generation_sha256())
            .collect::<BTreeSet<_>>()
            .len(),
        1
    );
    assert!(eligibility.layers().iter().all(|layer| {
        layer.transaction_provenance_sha256() == report.transaction_provenance_sha256()
    }));
    assert!(report.observations().iter().all(|observation| {
        eligibility.layers().iter().any(|layer| {
            layer.layer() == observation.layer()
                && layer.authority_root_sha256() == observation.authority_root_sha256()
        })
    }));
    assert_eq!(tree_snapshot(&repo.root), before_source);
    assert_eq!(tree_snapshot(&host.root), before_host);
}

#[test]
fn repeated_supported_observations_are_deterministic_but_session_provenance_is_unique() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);

    let first_session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut first_reader = exact_reader(&source, &host);
    first_session.verify(&mut first_reader).unwrap();
    let first = first_reader.report();

    let second_session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut second_reader = exact_reader(&source, &host);
    second_session.verify(&mut second_reader).unwrap();
    let second = second_reader.report();

    assert_eq!(first.generation_sha256(), second.generation_sha256());
    assert_eq!(first.observations(), second.observations());
    assert_eq!(first.findings(), second.findings());
    assert_ne!(
        first.transaction_provenance_sha256(),
        second.transaction_provenance_sha256()
    );
}

#[test]
fn supported_reader_reports_all_legacy_and_policy_authority_without_suppression() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    host.write(
        &host.global,
        ".codex/agents/harness_contract_claim_falsifier.toml",
        "name = \"harness_contract_claim_falsifier\"\ndescription = \"legacy\"\ndeveloper_instructions = \"legacy authority\"\n",
    );
    host.write(
        &host.global,
        ".codex/agents/claim_falsifier.toml",
        descriptor("claim_falsifier", "read-only"),
    );
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = exact_reader(&source, &host);

    assert_eq!(
        session.verify(&mut reader).unwrap_err().id(),
        AgentDiscoveryErrorId::CollidingAuthorityActive
    );
    let findings = reader.report().findings().to_vec();
    assert!(findings.iter().any(|finding| {
        finding.kind() == SupportedAgentAuthorityFindingKind::LegacyAuthority
            && finding.authority_name() == "harness_contract_claim_falsifier"
    }));
    assert!(findings.iter().any(|finding| {
        finding.kind() == SupportedAgentAuthorityFindingKind::SandboxPolicyMissing
            && finding.authority_name() == "harness_contract_claim_falsifier"
    }));
    assert!(findings.iter().any(|finding| {
        finding.kind() == SupportedAgentAuthorityFindingKind::NormalizedCollision
            && finding.authority_name() == "claim_falsifier"
    }));
}

#[test]
fn supported_reader_rejects_write_capable_unrelated_global_authority() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    host.write(
        &host.global,
        ".codex/agents/unrelated-observer.toml",
        descriptor("unrelated-observer", "workspace-write"),
    );
    let before_source = tree_snapshot(&repo.root);
    let before_host = tree_snapshot(&host.root);
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = exact_reader(&source, &host);

    let result = session.verify(&mut reader);
    let report = reader.report();

    assert_eq!(
        result.unwrap_err().id(),
        AgentDiscoveryErrorId::SandboxPolicyRejected
    );
    assert_eq!(report.effect_probe_count(), 0);
    assert_eq!(report.write_operation_count(), 0);
    assert!(report.findings().iter().any(|finding| {
        finding.layer() == AgentAuthorityLayer::Global
            && finding.kind() == SupportedAgentAuthorityFindingKind::WriteCapableSandbox
            && finding.authority_name() == "unrelated-observer"
    }));
    assert_eq!(tree_snapshot(&repo.root), before_source);
    assert_eq!(tree_snapshot(&host.root), before_host);
}

#[test]
fn supported_reader_observes_unrelated_read_only_global_authority_without_eligibility() {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let host = SupportedHostFixture::exact(&source);
    host.write(
        &host.global,
        ".codex/agents/unrelated-observer.toml",
        descriptor("unrelated-observer", "read-only"),
    );
    let before_source = tree_snapshot(&repo.root);
    let before_host = tree_snapshot(&host.root);
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = exact_reader(&source, &host);

    let eligibility = session.verify(&mut reader).unwrap();
    let report = reader.report();

    assert!(!eligibility.route_eligible());
    assert!(!eligibility.new_session_observed());
    assert!(!eligibility.has_claim_effect());
    assert_eq!(report.effect_probe_count(), 6);
    assert_eq!(report.write_operation_count(), 0);
    assert!(report.findings().is_empty());
    assert!(report.observations().iter().any(|observation| {
        observation.layer() == AgentAuthorityLayer::Global
            && observation.authority_name() == "unrelated-observer"
            && observation.sandbox_mode() == Some("read-only")
    }));
    assert_eq!(tree_snapshot(&repo.root), before_source);
    assert_eq!(tree_snapshot(&host.root), before_host);
}

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
    let reader_source = include_str!("../../src/plugin_product/agent_discovery/supported.rs");
    let module_wiring = include_str!("../../src/plugin_product/mod.rs");

    assert!(!reader_source.contains("new_session: true"));
    assert!(!reader_source.contains("std::fs::write"));
    assert!(!reader_source.contains("OpenOptions"));
    assert!(!reader_source.contains("File::create"));
    assert!(module_wiring.contains("pub(crate) mod agent_discovery;"));
    assert!(!module_wiring.contains("pub mod agent_discovery;"));
}
