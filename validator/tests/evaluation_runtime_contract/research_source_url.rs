fn research_source_url(source_id: &str) -> String {
    format!("https://primary.example/{source_id}")
}

fn research_record(
    source_id: &str,
    source_class: ResearchSourceClass,
    temporal_scope: FactTemporalScope,
    supported_proposals: BTreeSet<String>,
    observed_at_offset_seconds: u64,
) -> ResearchSourceRecord {
    let laws = research_laws();
    let url = research_source_url(source_id);
    complete_research_record(
        source_id,
        "Primary Example Publisher",
        source_class,
        "2026-07-13",
        research_epoch(observed_at_offset_seconds),
        vec![
            VerifiedSourceFact::new(
                "fact-current-capability",
                format!("Verified current capability fact from {source_id}."),
                source_id,
                format!("{url}#verified-fact"),
                laws.clone(),
                temporal_scope,
            )
            .unwrap(),
        ],
        supported_proposals,
    )
}

fn complete_research_record(
    source_id: &str,
    publisher: &str,
    source_class: ResearchSourceClass,
    checked_date: &str,
    observed_at_epoch_seconds: u64,
    verified_source_facts: Vec<VerifiedSourceFact>,
    supported_proposals: BTreeSet<String>,
) -> ResearchSourceRecord {
    let laws = research_laws();
    let url = research_source_url(source_id);
    ResearchSourceRecord::new(ResearchSourceRecordDefinition {
        source_id: source_id.to_owned(),
        publisher: publisher.to_owned(),
        url: url.clone(),
        source_class,
        checked_date: checked_date.to_owned(),
        observed_at_epoch_seconds,
        verified_source_facts,
        binding_product_requirements: vec![
            BindingProductRequirement::new(
                "REQ-RESEARCH-001",
                format!("Binding product requirement derived for {source_id}."),
                "HUL-RESEARCH-001",
            )
            .unwrap(),
        ],
        advisory_practices: vec![
            AdvisoryPractice::new(
                "practice-bounded-review",
                format!("Advisory practice retained separately for {source_id}."),
                source_id,
                format!("{url}#advisory-practice"),
                laws.clone(),
            )
            .unwrap(),
        ],
        experimental_hypotheses: vec![
            ExperimentalHypothesis::new(
                "hypothesis-review-latency",
                format!("Experimental hypothesis recorded for {source_id}."),
                source_id,
                format!("{url}#experimental-hypothesis"),
                laws.clone(),
                "Reject when the representative latency sample exceeds the adopted bound.",
            )
            .unwrap(),
        ],
        rejected_recommendations: vec![
            RejectedRecommendation::new(
                "recommendation-auto-adopt",
                format!("Rejected automatic-adoption recommendation from {source_id}."),
                source_id,
                format!("{url}#rejected-recommendation"),
                laws.clone(),
                "Only an explicit reviewed contract change may alter binding law.",
            )
            .unwrap(),
        ],
        limitations: vec![
            "The source does not prove installed or runtime product behavior.".to_owned(),
        ],
        mapped_law_ids: laws,
        supports_proposal_ids: supported_proposals,
    })
    .unwrap()
}

fn bind_research_record(relative_path: &str, record: &ResearchSourceRecord) -> ResearchSource {
    let bytes = record.canonical_bytes();
    ResearchSource::test_only_from_root_adopted_record(
        BoundInput::regular(relative_path, digest(&bytes), bytes.len() as u64),
        bytes,
    )
    .unwrap()
}

fn bind_untrusted_research_record(
    relative_path: &str,
    record: &ResearchSourceRecord,
) -> ResearchSource {
    let bytes = record.canonical_bytes();
    ResearchSource::from_bound_record(
        BoundInput::regular(relative_path, digest(&bytes), bytes.len() as u64),
        bytes,
    )
    .unwrap()
}

fn research_source(
    source_id: &str,
    supported_proposals: BTreeSet<String>,
    observed_at_offset_seconds: u64,
) -> ResearchSource {
    let record = research_record(
        source_id,
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::MutableCapability,
        supported_proposals,
        observed_at_offset_seconds,
    );
    bind_research_record(&format!("research/{source_id}.json"), &record)
}

fn audit_research(
    sources: &[ResearchSource],
    proposals: &[LawChangeProposal],
    current_offset_seconds: u64,
) -> ResearchAudit {
    ResearchAudit::test_only_audit_at(sources, proposals, research_epoch(current_offset_seconds))
}

fn proposal_analyses(
    supporting_sources: &BTreeSet<String>,
    mapped_laws: &BTreeSet<String>,
) -> ProposalAnalyses {
    ProposalAnalyses::new(
        ImpactAnalysis::new(
            "Impact is limited to the reviewed research contract surfaces.",
            BTreeSet::from(["REQ-RESEARCH-004".to_owned()]),
            BTreeSet::from(["research-law-change".to_owned()]),
            mapped_laws.clone(),
        )
        .unwrap(),
        MigrationAnalysis::new(
            "Migration requires a reviewed contract amendment before implementation.",
            vec!["Prepare and review a candidate-bound contract amendment.".to_owned()],
            vec![
                "Retain the prior binding law when review does not accept the amendment."
                    .to_owned(),
            ],
            mapped_laws.clone(),
        )
        .unwrap(),
        ProofAnalysis::new(
            "Proof must bind the reviewed amendment to current primary evidence.",
            BTreeSet::from(["REQ-RESEARCH-004-PROOF".to_owned()]),
            vec!["Substitute a source digest and require rejection.".to_owned()],
            mapped_laws.clone(),
        )
        .unwrap(),
        AuthorityAnalysis::root_review_required(
            "Only Ultra root may adopt a reviewed contract change.",
            supporting_sources.clone(),
            mapped_laws.clone(),
        )
        .unwrap(),
    )
}
