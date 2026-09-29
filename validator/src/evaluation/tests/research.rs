use super::super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const OBSERVED_AT: u64 = 1_767_225_600; // 2026-01-01T00:00:00Z

fn laws() -> BTreeSet<String> {
    BTreeSet::from(["HUL-TEST-001".to_owned()])
}

fn record() -> ResearchSourceRecord {
    let source_id = "source-primary";
    let url = "https://example.test/research";
    ResearchSourceRecord::new(ResearchSourceRecordDefinition {
        source_id: source_id.to_owned(),
        publisher: "Primary Research Publisher".to_owned(),
        url: url.to_owned(),
        source_class: ResearchSourceClass::PrimarySpecification,
        checked_date: "2026-01-01".to_owned(),
        observed_at_epoch_seconds: OBSERVED_AT,
        verified_source_facts: vec![
            VerifiedSourceFact::new(
                "fact-current-capability",
                "The evaluated capability is currently documented.",
                source_id,
                format!("{url}#fact-current-capability"),
                laws(),
                FactTemporalScope::MutableCapability,
            )
            .unwrap(),
        ],
        binding_product_requirements: vec![
            BindingProductRequirement::new(
                "requirement-evidence",
                "Evidence must remain independently reviewed.",
                "HUL-TEST-001",
            )
            .unwrap(),
        ],
        advisory_practices: vec![
            AdvisoryPractice::new(
                "practice-review",
                "Review observed evidence before adopting it.",
                source_id,
                format!("{url}#practice-review"),
                laws(),
            )
            .unwrap(),
        ],
        experimental_hypotheses: vec![
            ExperimentalHypothesis::new(
                "hypothesis-review",
                "Independent review reduces unsupported adoption.",
                source_id,
                format!("{url}#hypothesis-review"),
                laws(),
                "Compare independently reviewed outcomes.",
            )
            .unwrap(),
        ],
        rejected_recommendations: vec![
            RejectedRecommendation::new(
                "rejected-self-certification",
                "Allow sources to certify their own authority.",
                source_id,
                format!("{url}#rejected-self-certification"),
                laws(),
                "Caller-controlled records cannot establish authority.",
            )
            .unwrap(),
        ],
        limitations: vec!["This source is scoped to the documented capability.".to_owned()],
        mapped_law_ids: laws(),
        supports_proposal_ids: BTreeSet::from(["proposal-safe".to_owned()]),
    })
    .unwrap()
}

fn proposal() -> LawChangeProposal {
    let sources = BTreeSet::from(["source-primary".to_owned()]);
    LawChangeProposal::non_authoritative(
        "proposal-safe",
        "Require evidence review",
        "The source supports independent review controls.",
        sources.clone(),
        laws(),
        ProposalAnalyses::new(
            ImpactAnalysis::new(
                "Protect the evaluation adoption boundary.",
                BTreeSet::from(["REQ-EVIDENCE".to_owned()]),
                BTreeSet::from(["evaluation".to_owned()]),
                laws(),
            )
            .unwrap(),
            MigrationAnalysis::new(
                "Apply the review control deliberately.",
                vec!["Apply the bounded review control.".to_owned()],
                vec!["Restore the prior bounded control.".to_owned()],
                laws(),
            )
            .unwrap(),
            ProofAnalysis::new(
                "Prove both accepted and refused source paths.",
                BTreeSet::from(["proof-source-binding".to_owned()]),
                vec!["Reject externally asserted authority.".to_owned()],
                laws(),
            )
            .unwrap(),
            AuthorityAnalysis::root_review_required("Require root review.", sources, laws())
                .unwrap(),
        ),
    )
    .unwrap()
}

fn bound(record: &ResearchSourceRecord) -> (BoundInput, Vec<u8>) {
    let bytes = record.canonical_bytes();
    (
        BoundInput::regular(
            "research/source-primary.json",
            super::super::digest(&bytes),
            bytes.len() as u64,
        ),
        bytes,
    )
}

fn has(audit: &ResearchAudit, code: &str) -> bool {
    audit
        .findings()
        .iter()
        .any(|finding| finding.code() == code)
}

#[test]
fn untrusted_research_records_cannot_launder_their_own_authority() {
    let record = record();
    let (snapshot, bytes) = bound(&record);
    let external = ResearchSource::from_bound_record(snapshot, bytes).unwrap();
    let audit = ResearchAudit::test_only_audit_at(&[external], &[proposal()], OBSERVED_AT + 1);
    assert!(audit.eligible_proposals().is_empty());
    assert!(has(&audit, "research-source-authority-binding-required"));
    assert_eq!(audit.authority_effect(), "none");
}

#[test]
fn adopted_research_rejects_mutate_restore_and_classification_laundering() {
    let record = record();
    let (snapshot, bytes) = bound(&record);
    let adopted = ResearchSource::test_only_from_root_adopted_record(snapshot, bytes).unwrap();
    let accepted = ResearchAudit::test_only_audit_at(
        std::slice::from_ref(&adopted),
        &[proposal()],
        OBSERVED_AT + 1,
    );
    assert_eq!(accepted.eligible_proposals().len(), 1);

    for control in [
        "fact-advice-laundering",
        "requirement-hypothesis-laundering",
    ] {
        let mut mutated = adopted.clone();
        mutated.invalidate_classification_for_test(control);
        let audit = ResearchAudit::test_only_audit_at(&[mutated], &[proposal()], OBSERVED_AT + 1);
        assert!(audit.eligible_proposals().is_empty(), "{control}");
        assert!(has(&audit, "research-source-invalid"), "{control}");
    }
}

#[test]
fn research_read_parse_and_audit_are_zero_write() {
    let root = std::env::temp_dir().join(format!(
        "hul-evaluation-research-zero-write-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let path = root.join("source.json");
    let record = record();
    let bytes = record.canonical_bytes();
    fs::write(&path, &bytes).unwrap();
    let before = tree(&root);
    let source = ResearchSource::from_bound_record(
        BoundInput::regular(
            "research/source-primary.json",
            super::super::digest(&bytes),
            bytes.len() as u64,
        ),
        fs::read(&path).unwrap(),
    )
    .unwrap();
    let _ = ResearchAudit::test_only_audit_at(&[source], &[proposal()], OBSERVED_AT + 1);
    assert_eq!(tree(&root), before);
    fs::remove_dir_all(root).unwrap();
}

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}
