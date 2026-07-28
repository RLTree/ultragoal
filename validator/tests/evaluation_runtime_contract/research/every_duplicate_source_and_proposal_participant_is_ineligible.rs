#[test]
fn research_every_duplicate_source_and_proposal_participant_is_ineligible() {
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let duplicate_id_sources = [
        research_source(
            "source-primary",
            BTreeSet::from(["proposal-safe".to_owned()]),
            10,
        ),
        research_source(
            "source-primary",
            BTreeSet::from(["proposal-safe".to_owned()]),
            11,
        ),
    ];
    let audit = audit_research(&duplicate_id_sources, &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert_eq!(
        audit
            .findings()
            .iter()
            .filter(|finding| finding.code() == "research-source-duplicate")
            .count(),
        2
    );

    let duplicate = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let duplicate_digest_sources = [duplicate.clone(), duplicate];
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let audit = audit_research(&duplicate_digest_sources, &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert_eq!(
        audit
            .findings()
            .iter()
            .filter(|finding| finding.code() == "research-source-duplicate")
            .count(),
        2
    );

    let source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let duplicate_proposals = [
        research_proposal(
            "proposal-safe",
            BTreeSet::from(["source-primary".to_owned()]),
        ),
        research_proposal(
            "proposal-safe",
            BTreeSet::from(["source-primary".to_owned()]),
        ),
    ];
    let audit = audit_research(&[source], &duplicate_proposals, 50);
    assert!(audit.eligible_proposals().is_empty());
    assert_eq!(
        audit
            .findings()
            .iter()
            .filter(|finding| finding.code() == "research-proposal-duplicate")
            .count(),
        2
    );
}

#[test]
fn research_empty_and_oversized_sets_have_no_eligible_proposals() {
    let proposal = research_proposal(
        "proposal-safe",
        BTreeSet::from(["source-primary".to_owned()]),
    );
    let audit = audit_research(&[], &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-source-count-out-of-bounds")
    );

    let source = research_source(
        "source-primary",
        BTreeSet::from(["proposal-safe".to_owned()]),
        10,
    );
    let audit = audit_research(&[source.clone()], &[], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-proposal-count-out-of-bounds")
    );

    let oversized_sources = (0..129)
        .map(|index| {
            let source_id = format!("source-{index}");
            research_source(&source_id, BTreeSet::from(["proposal-safe".to_owned()]), 10)
        })
        .collect::<Vec<_>>();
    let proposal = research_proposal("proposal-safe", BTreeSet::from(["source-0".to_owned()]));
    let audit = audit_research(&oversized_sources, &[proposal], 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-source-count-out-of-bounds")
    );

    let oversized_proposals = (0..33)
        .map(|index| {
            research_proposal(
                &format!("proposal-{index}"),
                BTreeSet::from(["source-primary".to_owned()]),
            )
        })
        .collect::<Vec<_>>();
    let audit = audit_research(&[source], &oversized_proposals, 50);
    assert!(audit.eligible_proposals().is_empty());
    assert!(
        audit
            .findings()
            .iter()
            .any(|finding| finding.code() == "research-proposal-count-out-of-bounds")
    );
}

fn recursive_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, current: &Path, rows: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.file_type().is_dir() {
                visit(root, &path, rows);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let bytes = if metadata.file_type().is_symlink() {
                    fs::read_link(&path)
                        .unwrap()
                        .to_string_lossy()
                        .as_bytes()
                        .to_vec()
                } else {
                    fs::read(&path).unwrap()
                };
                rows.insert(relative, bytes);
            }
        }
    }

    let mut rows = BTreeMap::new();
    visit(root, root, &mut rows);
    rows
}

fn git_status(root: &Path) -> Vec<u8> {
    let output = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain=v1", "--untracked-files=all"])
        .output()
        .unwrap();
    assert!(output.status.success(), "git status failed: {output:?}");
    output.stdout
}
