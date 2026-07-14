fn research_proposal(proposal_id: &str, supporting_sources: BTreeSet<String>) -> LawChangeProposal {
    let mapped_laws = research_laws();
    LawChangeProposal::non_authoritative(
        proposal_id,
        format!("Proposal {proposal_id}"),
        format!("Current primary evidence supports review of {proposal_id}."),
        supporting_sources.clone(),
        mapped_laws.clone(),
        proposal_analyses(&supporting_sources, &mapped_laws),
    )
    .unwrap()
}

fn research_source_rejected(source: ResearchSource, expected_code: &str) {
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from([source.source_id().to_owned()]),
    );
    let audit = audit_research(&[source], &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == expected_code),
        "missing research finding {expected_code}: {:?}",
        audit.findings()
    );
}

fn research_untrusted_current_primary_rejected(source: ResearchSource) {
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from([source.source_id().to_owned()]),
    );
    let audit = audit_research(&[source], &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| { finding.code() == "research-source-authority-binding-required" })
    );
    for unexpected in [
        "research-source-invalid",
        "research-source-stable-fact-authority-required",
        "research-source-stale",
        "research-source-current-primary-required",
    ] {
        assert!(
            audit
                .findings()
                .iter()
                .all(|finding| finding.code() != unexpected),
            "untrusted current-primary control was rejected for {unexpected}: {:?}",
            audit.findings()
        );
    }
}

fn assert_only_no_authority_effects(value: &Value, effects: &mut usize) {
    match value {
        Value::Array(values) => {
            for value in values {
                assert_only_no_authority_effects(value, effects);
            }
        }
        Value::Object(fields) => {
            for (name, value) in fields {
                if name == "authority_effect" {
                    *effects += 1;
                    assert_eq!(value, "none");
                }
                assert_only_no_authority_effects(value, effects);
            }
        }
        _ => {}
    }
}

#[test]
fn research_current_supported_input_yields_only_a_non_authoritative_proposal() {
    let source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let record = serde_json::to_value(source.record()).unwrap();
    for field in [
        "publisher",
        "url",
        "source_class",
        "checked_date",
        "verified_source_facts",
        "binding_product_requirements",
        "advisory_practices",
        "experimental_hypotheses",
        "rejected_recommendations",
        "limitations",
        "mapped_law_ids",
    ] {
        assert!(
            record.get(field).is_some(),
            "missing typed source field {field}"
        );
    }
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let audit = audit_research(&[source], &[proposal], 50);

    assert!(audit.findings().is_empty());
    assert_eq!(audit.eligible_proposals().len(), 1);
    assert_eq!(audit.authority_effect(), "none");
    assert_eq!(audit.eligible_proposals()[0].authority_effect(), "none");

    let serialized = serde_json::to_value(&audit).unwrap();
    let mut effects = 0;
    assert_only_no_authority_effects(&serialized, &mut effects);
    assert_eq!(
        effects, 3,
        "audit, proposal, and authority analysis must each deny authority"
    );
}
