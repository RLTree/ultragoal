fn exact_reader(
    source: &super::super::SourceAgentCatalog,
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
