use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("json parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json bytes")).expect("write json");
}

fn fixture_anchors() -> Value {
    json!({
        "validator_receipt":"fixtures/review-round/anchors/validator-receipt.json",
        "review_target_receipt":"fixtures/review-round/anchors/review-target-receipt.json",
        "archive_receipt":"fixtures/review-round/anchors/archive-receipt.json"
    })
}

#[test]
fn review_round_red_fixture_observation_uses_anchor_overrides_and_first_failure() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let store = crate::schema_catalog::load(&root);
    let observation = crate::red::fixture::review::round::observation(
        &root,
        &store,
        &json!({"review_round_anchor_overrides": fixture_anchors()}),
        &json!({
            "check_id":"validator-execution-provenance",
            "error":"not-the-first-failure"
        }),
        &json!({"schema":"wrong"}),
    );
    assert!(!observation.ok);
    assert!(!observation.check.is_empty(), "{:?}", observation.error);
    assert!(!observation.error.is_empty());
}

#[test]
fn red_fixture_results_cover_patch_missing_invalid_and_materialized_rows() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("red-fixture-result-branches");
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    write_json(
        &root.join("fixtures/valid/minimal-goal-run.json"),
        &json!({
            "claims":[{
                "id":"CLAIM-RED",
                "status":"pass",
                "claim_ceiling_effect":"included",
                "evidence":[{
                    "kind":"test_pass",
                    "surface":"ci",
                    "digest":crate::self_tests::boundaries::workspace_fixtures::sha('d')
                }]
            }]
        }),
    );
    write_json(
        &root.join("fixtures/red/missing-patch.json"),
        &json!({
            "expected_failure":{
                "check_id":"red-fixture-coverage",
                "error":"red_fixture_json_patch_missing"
            },
            "base_fixture_path":"fixtures/valid/minimal-goal-run.json"
        }),
    );
    write_json(
        &root.join("fixtures/red/bad-pointer.json"),
        &json!({
            "expected_failure":{
                "check_id":"red-fixture-coverage",
                "error":"red_fixture_json_pointer_invalid"
            },
            "base_fixture_path":"fixtures/valid/minimal-goal-run.json",
            "json_patch":[{"op":"replace","path":"/claims/99/id","value":"NOPE"}]
        }),
    );
    write_json(
        &root.join("fixtures/red/materialized.json"),
        &json!({
            "expected_failure":{
                "check_id":"claim-evidence-boundary",
                "error":"claim_evidence_missing_path"
            },
            "materialization":{"first_failure_must_match_expected":false},
            "base_fixture_path":"fixtures/valid/minimal-goal-run.json",
            "json_patch":[]
        }),
    );
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([
            {
                "id":"missing-patch",
                "packet_path":"fixtures/red/missing-patch.json",
                "expected_failure":{
                    "check_id":"red-fixture-coverage",
                    "error":"red_fixture_json_patch_missing"
                }
            },
            {
                "id":"bad-pointer",
                "packet_path":"fixtures/red/bad-pointer.json",
                "expected_failure":{
                    "check_id":"red-fixture-coverage",
                    "error":"red_fixture_json_pointer_invalid"
                }
            },
            {
                "id":"materialized",
                "packet_path":"fixtures/red/materialized.json",
                "expected_failure":{
                    "check_id":"claim-evidence-boundary",
                    "error":"claim_evidence_missing_path"
                }
            }
        ]),
    );

    let results = crate::red::fixtures::red_fixture_results(&root, &store, &BTreeMap::new());
    assert_eq!(
        results["missing-patch"]["observed_error"],
        "red_fixture_json_patch_missing"
    );
    assert_eq!(
        results["bad-pointer"]["observed_error"],
        "red_fixture_json_pointer_invalid"
    );
    assert!(
        !results["materialized"]["observed_error"]
            .as_str()
            .unwrap_or("")
            .is_empty()
    );
    std::fs::remove_dir_all(root).expect("cleanup red fixture result branches");
}

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
