#[test]
fn research_read_parse_query_and_audit_paths_are_recursively_zero_write() {
    let repository = root("research-zero-write");
    let initialized = Command::new("git")
        .args(["init", "-q"])
        .arg(&repository)
        .status()
        .unwrap();
    assert!(initialized.success());
    let research_dir = repository.join("research");
    fs::create_dir(&research_dir).unwrap();
    let record = research_record(
        "source-primary",
        ResearchSourceClass::PrimarySpecification,
        FactTemporalScope::VersionClaim,
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let source_path = research_dir.join("source-primary.json");
    fs::write(&source_path, record.canonical_bytes()).unwrap();
    let before_status = git_status(&repository);
    let before_tree = recursive_tree(&repository);

    let bytes = fs::read(&source_path).unwrap();
    let source = ResearchSource::from_bound_record(
        BoundInput::regular(
            "research/source-primary.json",
            digest(&bytes),
            bytes.len() as u64,
        ),
        bytes,
    )
    .unwrap();
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let audit = audit_research(&[source], &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| { finding.code() == "research-source-authority-binding-required" })
    );

    assert_eq!(git_status(&repository), before_status);
    assert_eq!(recursive_tree(&repository), before_tree);
    fs::remove_dir_all(repository).unwrap();
}

fn data_controls(
    success: &str,
    failure: &str,
    split: &str,
    training_splits: BTreeSet<String>,
    semantic: char,
    near: char,
    known_training: BTreeSet<String>,
    declared: &str,
    verified: &str,
    sampled: &str,
    target: &str,
) -> EvaluationDataControls {
    EvaluationDataControls::new(EvaluationDataControlsDefinition {
        objective: "observe-core".to_owned(),
        success_criterion: success.to_owned(),
        failure_criterion: failure.to_owned(),
        split_id: split.to_owned(),
        training_split_ids: training_splits,
        semantic_fingerprint_sha256: sha(semantic),
        near_duplicate_group_sha256: sha(near),
        known_training_fingerprint_sha256s: known_training,
        declared_label: declared.to_owned(),
        verified_label: verified.to_owned(),
        sampled_population: sampled.to_owned(),
        target_population: target.to_owned(),
    })
}

#[test]
fn ambiguity_split_near_duplicate_label_and_population_leakage_reject() {
    let cases = [
        (
            data_controls(
                "same",
                "same",
                "eval",
                BTreeSet::new(),
                '1',
                '2',
                BTreeSet::new(),
                "pass",
                "pass",
                "target",
                "target",
            ),
            "evaluation-task-ambiguous",
        ),
        (
            data_controls(
                "pass",
                "fail",
                "eval",
                BTreeSet::from(["eval".to_owned()]),
                '1',
                '2',
                BTreeSet::new(),
                "pass",
                "pass",
                "target",
                "target",
            ),
            "evaluation-split-leakage-detected",
        ),
        (
            data_controls(
                "pass",
                "fail",
                "eval",
                BTreeSet::new(),
                '1',
                '2',
                BTreeSet::from([sha('2')]),
                "pass",
                "pass",
                "target",
                "target",
            ),
            "evaluation-near-duplicate-leakage-detected",
        ),
        (
            data_controls(
                "pass",
                "fail",
                "eval",
                BTreeSet::new(),
                '1',
                '2',
                BTreeSet::new(),
                "pass",
                "fail",
                "target",
                "target",
            ),
            "evaluation-label-mismatch",
        ),
        (
            data_controls(
                "pass",
                "fail",
                "eval",
                BTreeSet::new(),
                '1',
                '2',
                BTreeSet::new(),
                "pass",
                "pass",
                "sample",
                "target",
            ),
            "evaluation-unrepresentative-data",
        ),
    ];
    for (controls, expected) in cases {
        let spec = EvaluationSpec::new(
            sha('a'),
            sha('b'),
            "leakage-suite",
            vec![task("core", 'd').with_data_controls(controls)],
        )
        .unwrap();
        assert!(
            spec.audit(&sha('a'), &sha('b'))
                .findings()
                .iter()
                .any(|finding| finding == expected),
            "missing {expected}"
        );
    }
}
