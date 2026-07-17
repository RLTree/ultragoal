use serde_json::json;

#[test]
fn review_round_roles_reject_reuse_duplicates_unknown_and_manifest_substitutes() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let anchors = crate::review::round::anchor::values::AnchorValues {
        validator_path: "validator.json".into(),
        review_target_path: "target.json".into(),
        archive_path: "archive.json".into(),
        validator_digest: crate::self_tests::boundaries::workspace_fixtures::sha('1'),
        review_target_digest: crate::self_tests::boundaries::workspace_fixtures::sha('2'),
        archive_digest: crate::self_tests::boundaries::workspace_fixtures::sha('3'),
        validator_run_id: "run".into(),
        package_digest: crate::self_tests::boundaries::workspace_fixtures::sha('4'),
        source_errors: Vec::new(),
        materiality_anchors: Vec::new(),
    };
    let receipt = json!({
        "prior_round_reviewer_agent_ids":["prior-agent","dupe-agent"],
        "reviewers":[
            {
                "role":"claim-falsifier",
                "reviewer_agent_id":"dupe-agent",
                "agent_manifest_path":"agents/contract-claim-falsifier.md",
                "agent_manifest_digest":crate::self_tests::boundaries::workspace_fixtures::sha('a')
            },
            {
                "role":"claim-falsifier",
                "reviewer_agent_id":"dupe-agent",
                "agent_manifest_path":"custom-agents/harness-contract-claim-falsifier.toml",
                "agent_manifest_digest":crate::self_tests::boundaries::workspace_fixtures::sha('c')
            },
            {"role":"unknown-role","reviewer_agent_id":"prior-agent"}
        ]
    });
    let mut out = Vec::new();
    crate::review::round::personas::persona_errors(&root, &receipt, &anchors, &mut out);
    let errors = out
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    for expected in [
        "review_round_wrong_role_count",
        "review_round_missing_role",
        "review_round_reused_reviewer",
        "review_round_duplicate_role",
        "review_round_agent_manifest_mismatch",
    ] {
        assert!(errors.contains(&expected), "{expected}: {errors:?}");
    }
}

#[test]
fn review_round_roles_reject_missing_canonical_manifest_files() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("review-role-missing-files");
    std::fs::create_dir_all(&root).expect("root");
    let anchors = crate::review::round::anchor::values::AnchorValues {
        validator_path: "validator.json".into(),
        review_target_path: "target.json".into(),
        archive_path: "archive.json".into(),
        validator_digest: crate::self_tests::boundaries::workspace_fixtures::sha('1'),
        review_target_digest: crate::self_tests::boundaries::workspace_fixtures::sha('2'),
        archive_digest: crate::self_tests::boundaries::workspace_fixtures::sha('3'),
        validator_run_id: "run".into(),
        package_digest: crate::self_tests::boundaries::workspace_fixtures::sha('4'),
        source_errors: Vec::new(),
        materiality_anchors: Vec::new(),
    };
    let receipt = json!({
        "prior_round_reviewer_agent_ids":[],
        "reviewers":[{
            "role":"claim-falsifier",
            "reviewer_agent_id":"fresh-agent",
            "agent_manifest_path":".codex/agents/claim-falsifier.toml",
            "agent_manifest_digest":crate::self_tests::boundaries::workspace_fixtures::sha('a')
        }]
    });
    let mut out = Vec::new();
    crate::review::round::personas::persona_errors(&root, &receipt, &anchors, &mut out);
    let errors = out
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(
        errors.contains(&"review_round_agent_manifest_mismatch"),
        "{errors:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup missing role files");
}
