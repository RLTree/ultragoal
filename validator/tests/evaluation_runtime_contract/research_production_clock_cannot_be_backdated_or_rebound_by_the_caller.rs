#[test]
fn research_production_clock_cannot_be_backdated_or_rebound_by_the_caller() {
    let source_id = "source-old-primary";
    let source_url = research_source_url(source_id);
    let old_record = complete_research_record(
        source_id,
        "Primary Example Publisher",
        ResearchSourceClass::PrimarySpecification,
        "2026-01-01",
        RESEARCH_2026_01_01_EPOCH,
        vec![
            VerifiedSourceFact::new(
                "fact-old-capability",
                "Model X was observed as available on the checked date.",
                source_id,
                format!("{source_url}#old-capability"),
                research_laws(),
                FactTemporalScope::MutableCapability,
            )
            .unwrap(),
        ],
        BTreeSet::from(["proposal-safe".to_owned()]),
    );
    let old_source = bind_research_record("research/source-old-primary.json", &old_record);
    let proposal = research_proposal("proposal-safe", BTreeSet::from([source_id.to_owned()]));

    let simulated_backdate = ResearchAudit::test_only_audit_at(
        std::slice::from_ref(&old_source),
        std::slice::from_ref(&proposal),
        RESEARCH_2026_01_01_EPOCH + 1,
    );
    assert_eq!(simulated_backdate.eligible_proposals().len(), 1);
    assert!(simulated_backdate.findings().is_empty());

    let production = ResearchAudit::audit(
        std::slice::from_ref(&old_source),
        std::slice::from_ref(&proposal),
    );
    assert!(production.eligible_proposals().is_empty());
    assert!(
        production
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-source-stale"),
        "production clock accepted backdated source: {:?}",
        production.findings()
    );

    let future_now = ResearchAudit::test_only_audit_at(
        std::slice::from_ref(&old_source),
        std::slice::from_ref(&proposal),
        RESEARCH_CHECKED_DAY_EPOCH + 365 * 86_400,
    );
    assert!(future_now.eligible_proposals().is_empty());
    assert!(
        future_now
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-source-stale")
    );

    for invalid_now in [0, u64::MAX] {
        let audit = ResearchAudit::test_only_audit_at(
            std::slice::from_ref(&old_source),
            std::slice::from_ref(&proposal),
            invalid_now,
        );
        assert!(audit.eligible_proposals().is_empty());
        assert_eq!(audit.findings().len(), 1);
        assert_eq!(audit.findings()[0].code(), "research-clock-invalid");
        assert_eq!(audit.findings()[0].authority_effect(), "none");
    }

    let clock_failure = ResearchAudit::test_only_audit_with_clock_failure(
        std::slice::from_ref(&old_source),
        std::slice::from_ref(&proposal),
    );
    assert!(clock_failure.eligible_proposals().is_empty());
    assert_eq!(clock_failure.findings().len(), 1);
    assert_eq!(
        clock_failure.findings()[0].code(),
        "research-clock-unavailable"
    );
    assert_eq!(clock_failure.findings()[0].authority_effect(), "none");

    let mut rebound = serde_json::to_value(&old_record).unwrap();
    rebound["valid_until_epoch_seconds"] =
        json!(RESEARCH_CHECKED_DAY_EPOCH + RESEARCH_MUTABLE_FRESHNESS_SECONDS);
    let rebound: ResearchSourceRecord = serde_json::from_value(rebound).unwrap();
    let rebound_bytes = rebound.canonical_bytes();
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-old-primary.json",
                digest(&rebound_bytes),
                rebound_bytes.len() as u64,
            ),
            rebound_bytes,
        )
        .is_err(),
        "same-record validity rebound passed after exact digest replacement"
    );
}

#[test]
fn research_incomplete_or_substituted_proposal_analyses_never_become_authority() {
    let source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    for analysis in ["impact", "migration", "proof", "authority"] {
        let mut incomplete = proposal.clone();
        incomplete.test_only_clear_analysis(analysis);
        let audit = audit_research(&[source.clone()], &[incomplete], 50);
        assert!(audit.eligible_proposals().is_empty());
        assert!(audit.findings().iter().any(|finding| {
            finding.code() == "research-proposal-invalid" && finding.authority_effect() == "none"
        }));
        assert_eq!(audit.authority_effect(), "none");
    }
    for control in ["decision", "reviewer"] {
        let mut substituted = proposal.clone();
        substituted.test_only_substitute_authority(control);
        let audit = audit_research(&[source.clone()], &[substituted], 50);
        assert!(audit.eligible_proposals().is_empty());
        assert!(
            audit
                .findings()
                .iter()
                .any(|finding| finding.code() == "research-proposal-invalid")
        );
    }

    let mut source_substitution = proposal.clone();
    source_substitution
        .test_only_substitute_supporting_sources(BTreeSet::from(["source-substituted".to_owned()]));
    let audit = audit_research(&[source.clone()], &[source_substitution], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .all(|finding| finding.authority_effect() == "none")
    );

    let mut law_substitution = proposal;
    law_substitution.test_only_substitute_mapped_laws(BTreeSet::from(["HUL-OTHER-001".to_owned()]));
    let audit = audit_research(&[source], &[law_substitution], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .all(|finding| finding.authority_effect() == "none")
    );
}
