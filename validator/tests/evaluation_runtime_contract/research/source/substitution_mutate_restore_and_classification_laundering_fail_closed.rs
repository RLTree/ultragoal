#[test]
fn research_source_substitution_mutate_restore_and_classification_laundering_fail_closed() {
    let source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );

    let mut url_substitution = source.clone();
    url_substitution.substitute_url_for_test("https://substituted.example/source-primary");
    research_source_rejected(url_substitution, "research-source-invalid");

    let mut mutate_restore = source.clone();
    mutate_restore.substitute_url_for_test("https://substituted.example/source-primary");
    mutate_restore.substitute_url_for_test(research_source_url("source-primary"));
    research_source_rejected(mutate_restore, "research-source-invalid");

    let mut snapshot_substitution = source.clone();
    snapshot_substitution.substitute_snapshot_for_test(BoundInput::regular(
        "research/source-primary.json",
        sha('e'),
        source.record().canonical_bytes().len() as u64,
    ));
    research_source_rejected(snapshot_substitution, "research-source-invalid");

    let mut byte_substitution = source.clone();
    byte_substitution.substitute_record_bytes_for_test(source.record().canonical_bytes());
    research_source_rejected(byte_substitution, "research-source-invalid");

    for control in [
        "checked-date",
        "source-class",
        "limitation",
        "fact",
        "mapped-law",
    ] {
        let mut substituted = source.clone();
        substituted.substitute_typed_field_for_test(control);
        research_source_rejected(substituted, "research-source-invalid");
    }

    for control in [
        "fact-source-substitution",
        "fact-advice-laundering",
        "requirement-advice-laundering",
        "fact-hypothesis-laundering",
        "requirement-hypothesis-laundering",
    ] {
        let mut classified = source.clone();
        classified.invalidate_classification_for_test(control);
        research_source_rejected(classified, "research-source-invalid");
    }
}

#[test]
fn research_freshness_primary_source_and_support_mapping_fail_closed() {
    for current_epoch_seconds in [0, 9, RESEARCH_MUTABLE_FRESHNESS_SECONDS + 11] {
        let source = research_source(
            "source-primary",
            BTreeSet::from(["proposal-safe".to_owned()]),
            10,
        );
        let proposal = research_proposal(
            "proposal-safe",
            BTreeSet::from(["source-primary".to_owned()]),
        );
        let audit = audit_research(&[source], &[proposal], current_epoch_seconds);
        assert!(audit.eligible_proposals().is_empty());
        assert!(
            audit
                .findings()
                .iter()
                .any(|finding| finding.code() == "research-source-stale")
        );
    }

    let current_source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let boundary = audit_research(
        &[current_source.clone()],
        &[proposal.clone()],
        10 + RESEARCH_MUTABLE_FRESHNESS_SECONDS,
    );
    assert!(boundary.findings().is_empty());
    assert_eq!(boundary.eligible_proposals().len(), 1);
    let expired = audit_research(
        &[current_source.clone()],
        &[proposal],
        11 + RESEARCH_MUTABLE_FRESHNESS_SECONDS,
    );
    assert!(expired.eligible_proposals().is_empty());
    assert!(
        expired
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-source-stale")
    );

    for control in [
        "old-checked-date-current-observation",
        "source-declared-indefinite-validity",
    ] {
        let mut invalid = current_source.clone();
        invalid.invalidate_freshness_for_test(control);
        research_source_rejected(invalid, "research-source-invalid");
    }

    let mut non_primary = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    non_primary.substitute_class_for_test(ResearchSourceClass::VendorDocumentation);
    research_source_rejected(non_primary, "research-source-current-primary-required");

    let unsupported = research_source(
        "source-primary",
        BTreeSet::from(["proposal-other".to_owned()]),
        10,
    );
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let audit = audit_research(&[unsupported], &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-proposal-support-invalid")
    );
}
