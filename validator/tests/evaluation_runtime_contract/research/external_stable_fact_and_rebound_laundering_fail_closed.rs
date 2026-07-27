#[test]
fn research_external_stable_fact_and_rebound_laundering_fail_closed() {
    let source_id = "source-vendor-model-x";
    let source_url = research_source_url(source_id);
    let supported_proposals = BTreeSet::from(["proposal-safe".to_owned()]);
    let exact_counterexample = complete_research_record(
        source_id,
        "Model X Vendor",
        ResearchSourceClass::VendorDocumentation,
        "2026-01-01",
        RESEARCH_2026_01_01_EPOCH,
        vec![
            VerifiedSourceFact::new(
                "fact-model-x-current",
                "Model X is currently available in the product.",
                source_id,
                format!("{source_url}#model-x-current"),
                research_laws(),
                FactTemporalScope::Stable,
            )
            .unwrap(),
        ],
        supported_proposals.clone(),
    );
    let source = bind_untrusted_research_record(
        "research/source-vendor-model-x.json",
        &exact_counterexample,
    );
    let proposal = research_proposal("proposal-safe", BTreeSet::from([source_id.to_owned()]));
    let audit = ResearchAudit::test_only_audit_at(
        std::slice::from_ref(&source),
        std::slice::from_ref(&proposal),
        RESEARCH_CHECKED_DAY_EPOCH,
    );
    assert!(audit.eligible_proposals().is_empty());
    for expected in [
        "research-source-authority-binding-required",
        "research-source-stable-fact-authority-required",
        "research-source-stale",
        "research-source-current-primary-required",
    ] {
        assert!(
            audit
                .findings()
                .iter()
                .any(|finding| finding.code() == expected),
            "missing exact laundering finding {expected}: {:?}",
            audit.findings()
        );
    }
    assert!(
        audit
            .findings()
            .iter()
            .all(|finding| finding.authority_effect() == "none")
    );
    assert_eq!(audit.authority_effect(), "none");

    let current_record = research_record(
        "source-primary",
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::MutableCapability,
        supported_proposals.clone(),
        10,
    );
    let externally_constructed_current =
        bind_untrusted_research_record("research/source-primary.json", &current_record);
    research_untrusted_current_primary_rejected(externally_constructed_current);

    let class_only_base = research_record(
        "source-class-only",
        ResearchSourceClass::VendorDocumentation,
        FactTemporalScope::MutableCapability,
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let mut source_class_only_rebound = serde_json::to_value(class_only_base).unwrap();
    source_class_only_rebound["source_class"] = json!("primary_specification");
    let source_class_only_rebound: ResearchSourceRecord =
        serde_json::from_value(source_class_only_rebound).unwrap();
    research_untrusted_current_primary_rejected(bind_untrusted_research_record(
        "research/source-class-only.json",
        &source_class_only_rebound,
    ));

    let temporal_only_base = research_record(
        "source-temporal-only",
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::Stable,
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let mut temporal_only_rebound = serde_json::to_value(temporal_only_base).unwrap();
    temporal_only_rebound["verified_source_facts"][0]["temporal_scope"] =
        json!("mutable_capability");
    let temporal_only_rebound: ResearchSourceRecord =
        serde_json::from_value(temporal_only_rebound).unwrap();
    research_untrusted_current_primary_rejected(bind_untrusted_research_record(
        "research/source-temporal-only.json",
        &temporal_only_rebound,
    ));

    let observation_only_base = complete_research_record(
        "source-observation-only",
        "Primary Example Publisher",
        ResearchSourceClass::PrimarySpecification,
        "2026-01-01",
        RESEARCH_2026_01_01_EPOCH,
        vec![
            VerifiedSourceFact::new(
                "fact-observation-only",
                "Model X was observed as available on the checked date.",
                "source-observation-only",
                format!(
                    "{}#observation-only",
                    research_source_url("source-observation-only")
                ),
                research_laws(),
                FactTemporalScope::MutableCapability,
            )
            .unwrap(),
        ],
        BTreeSet::from(["proposal-safe".to_owned()]),
    );
    let mut observation_only_rebound = serde_json::to_value(observation_only_base).unwrap();
    observation_only_rebound["checked_date"] = json!("2026-07-13");
    observation_only_rebound["observed_at_epoch_seconds"] = json!(research_epoch(10));
    observation_only_rebound["valid_until_epoch_seconds"] =
        json!(research_epoch(10) + RESEARCH_MUTABLE_FRESHNESS_SECONDS);
    let observation_only_rebound: ResearchSourceRecord =
        serde_json::from_value(observation_only_rebound).unwrap();
    research_untrusted_current_primary_rejected(bind_untrusted_research_record(
        "research/source-observation-only.json",
        &observation_only_rebound,
    ));

    let mut source_class_rebound = serde_json::to_value(&current_record).unwrap();
    source_class_rebound["source_class"] = json!("vendor_documentation");
    let source_class_rebound: ResearchSourceRecord =
        serde_json::from_value(source_class_rebound).unwrap();
    let source_class_rebound =
        bind_untrusted_research_record("research/source-primary.json", &source_class_rebound);
    research_source_rejected(
        source_class_rebound,
        "research-source-current-primary-required",
    );

    let mut label_rebound = serde_json::to_value(&current_record).unwrap();
    label_rebound["verified_source_facts"][0]["temporal_scope"] = json!("stable");
    let label_rebound: ResearchSourceRecord = serde_json::from_value(label_rebound).unwrap();
    let label_rebound =
        bind_untrusted_research_record("research/source-primary.json", &label_rebound);
    research_source_rejected(
        label_rebound,
        "research-source-stable-fact-authority-required",
    );

    let mixed_record = complete_research_record(
        "source-mixed",
        "Primary Example Publisher",
        ResearchSourceClass::PrimarySpecification,
        "2026-07-13",
        research_epoch(10),
        vec![
            VerifiedSourceFact::new(
                "fact-current-capability",
                "Model X is currently available in the product.",
                "source-mixed",
                format!("{}#model-x-current", research_source_url("source-mixed")),
                research_laws(),
                FactTemporalScope::MutableCapability,
            )
            .unwrap(),
            VerifiedSourceFact::new(
                "fact-attempted-stable",
                "The same caller also labels a second fact stable.",
                "source-mixed",
                format!("{}#attempted-stable", research_source_url("source-mixed")),
                research_laws(),
                FactTemporalScope::Stable,
            )
            .unwrap(),
        ],
        supported_proposals,
    );
    let mixed_source = bind_untrusted_research_record("research/source-mixed.json", &mixed_record);
    research_source_rejected(
        mixed_source,
        "research-source-stable-fact-authority-required",
    );
    let mixed_audit = audit_research(
        &[bind_untrusted_research_record(
            "research/source-mixed.json",
            &mixed_record,
        )],
        &[research_proposal(
            "proposal-safe",
            BTreeSet::from(["source-mixed".to_owned()]),
        )],
        50,
    );
    assert!(
        mixed_audit
            .findings()
            .iter()
            .any(|finding| { finding.code() == "research-source-authority-binding-required" })
    );

    let mut self_consistent_substitution = serde_json::to_value(&exact_counterexample).unwrap();
    self_consistent_substitution["source_class"] = json!("primary_specification");
    self_consistent_substitution["verified_source_facts"][0]["temporal_scope"] =
        json!("mutable_capability");
    self_consistent_substitution["checked_date"] = json!("2026-07-13");
    self_consistent_substitution["observed_at_epoch_seconds"] = json!(research_epoch(10));
    self_consistent_substitution["valid_until_epoch_seconds"] =
        json!(research_epoch(10) + RESEARCH_MUTABLE_FRESHNESS_SECONDS);
    let self_consistent_substitution: ResearchSourceRecord =
        serde_json::from_value(self_consistent_substitution).unwrap();
    let self_consistent_substitution = bind_untrusted_research_record(
        "research/source-vendor-model-x.json",
        &self_consistent_substitution,
    );
    assert_ne!(
        self_consistent_substitution.snapshot().digest_sha256(),
        source.snapshot().digest_sha256(),
        "combined rebound did not replace the canonical source digest"
    );
    research_untrusted_current_primary_rejected(self_consistent_substitution);

    let mut extended_validity = serde_json::to_value(&exact_counterexample).unwrap();
    extended_validity["valid_until_epoch_seconds"] =
        json!(RESEARCH_2026_01_01_EPOCH + 365 * 86_400);
    let extended_validity: ResearchSourceRecord =
        serde_json::from_value(extended_validity).unwrap();
    let extended_bytes = extended_validity.canonical_bytes();
    assert!(
        ResearchSource::from_bound_record(
            BoundInput::regular(
                "research/source-vendor-model-x.json",
                digest(&extended_bytes),
                extended_bytes.len() as u64,
            ),
            extended_bytes,
        )
        .is_err(),
        "self-consistent stable-validity extension passed after canonical-byte rebound"
    );
}
