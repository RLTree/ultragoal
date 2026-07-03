use serde_json::json;
use std::collections::BTreeMap;

fn errors(out: &[crate::audit::contract::Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

#[test]
fn semantic_review_and_red_observation_edges_cover_success_paths() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "session_and_review-semantic-review-red",
    );
    std::fs::create_dir_all(&root).expect("root");
    let title = "Backend runtime verifier";
    let description = "CLI backend runtime proof.";
    let text = crate::claim::text::normalized_text(&[title, description]);
    let mut receipt = json!({
        "schema": "harness-ultragoal.semantic-classification-receipt.v1",
        "claim_id": "CLAIM-HUMAN",
        "canonical_text_digest": crate::digest::bytes(text.as_bytes()),
        "classifier_contract_id": "ultragoal-semantic-classification",
        "classifier_contract_version": "v1",
        "classifier_implementation_kind": "human_reviewer",
        "generated_at": "2026-06-25T00:00:00Z",
        "producer_actor_id": "producer",
        "classifier_actor_id": "reviewer",
        "actor_disjoint": true,
        "detected_semantic_classes": ["runtime_cli_backend_only_engine_only"],
        "rationale": "human review attestation for runtime-only semantics",
        "confidence": 0.99,
        "ambiguity": false,
        "classifier_evidence": {
            "evidence_type":"human_review_attestation",
            "digest":crate::self_tests::boundaries::workspace_fixtures::sha('h'),
            "summary":"human reviewer attestation"
        },
        "required_proof_gates": [],
        "claim_ceiling_recommendation": "withhold",
        "receipt_digest": crate::digest::ZERO
    });
    receipt["receipt_digest"] = json!(crate::digest::canonical_json(&receipt));
    let claim = json!({
        "id":"CLAIM-HUMAN",
        "title":title,
        "description":description,
        "status":"pass",
        "claim_ceiling_effect":"included",
        "semantic_classification_receipts":[receipt]
    });
    let mut semantic = Vec::new();
    crate::claim_semantics::semantic::receipt::policy::check_semantic_receipts(
        &claim,
        &json!({"claim_ids":["CLAIM-HUMAN"]}),
        &root,
        &mut semantic,
    );
    assert!(
        !errors(&semantic).contains(&"semantic_classification_receipt_malformed"),
        "{semantic:?}"
    );

    let repo = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let store = crate::schema_catalog::load(&repo);
    let packet =
        json!({"json_patch":[{"op":"replace","path":"/verification_backlog/rows","value":[]}]});
    let bad = json!({
        "completion_manifest":{"claims":[{
            "id":"CLAIM-RED",
            "status":"proven_static",
            "claim_ceiling_effect":"included",
            "evidence":[{"kind":"test_pass","surface":"ci","digest":crate::self_tests::boundaries::workspace_fixtures::sha('r')}]
        }]},
        "lane_registry":{"lanes":[{
            "id":"LANE-RED",
            "workspace":"workspace",
            "branch":"branch",
            "base_commit":"base",
            "current_commit":"commit",
            "target_branch":"main",
            "target_head_at_launch":"head",
            "target_head_at_validation":"head",
            "merge_base_at_validation":"base",
            "execplan":"EXECPLAN.md",
            "claim_ids":["CLAIM-RED"],
            "owned_paths":[]
        }],"root_verification_stages":[]},
        "ready_for_merge":{
            "ready":true,
            "lane_id":"LANE-RED",
            "workspace":"workspace",
            "branch":"branch",
            "base_commit":"base",
            "commit":"commit",
            "target_branch":"main",
            "target_head_at_launch":"head",
            "target_head_at_validation":"head",
            "merge_base":"base",
            "execplan":"EXECPLAN.md",
            "claim_ids":["CLAIM-RED"],
            "changed_files":[],
            "worktree_clean":true,
            "teardown_ready":true,
            "commands":[{"id":"ok","exit":0}]
        },
        "verification_backlog":{
            "schema":"harness-ultragoal.verification-backlog.v1",
            "manifest_digest":crate::self_tests::boundaries::workspace_fixtures::sha('a'),
            "generated_at":"2026-06-25T00:00:00Z",
            "rows":[]
        },
        "automation_tick_receipt":{
            "freshness_policy":{
                "clock_at_validation":"2026-06-25T00:00:00Z",
                "max_tick_age_minutes":10,
                "max_success_age_minutes":10
            },
            "last_tick_at":"2026-06-25T00:00:00Z",
            "last_success_at":"2026-06-25T00:00:00Z",
            "validator_computed_drift_verdict":"fresh",
            "drift_verdict":"fresh"
        }
    });
    let schema_errors = crate::red::fixture::schema::errors(&store, &packet, &bad);
    assert!(schema_errors.is_empty(), "{schema_errors:?}");
    let observed = crate::red::fixture::observation::observe_materialized(
        &repo,
        &store,
        &BTreeMap::new(),
        &packet,
        &json!({"check_id":"claim-evidence-coupling","error":"proof_surface_substitution"}),
        &bad,
        "fixtures/valid/semantic-claim-boundary.json",
    );
    assert!(
        observed.ok,
        "check={} error={}",
        observed.check, observed.error
    );

    let mut ceiling = crate::self_tests::review::claim_ceiling::receipt();
    ceiling["claim_ceiling"]["unsupported"] = json!([
        {"claim_id":"plugins_ui_visibility"},
        {"claim_id":"install_button_success"},
        {"claim_id":"workspace_public_marketplace_publication"},
        {"claim_id":"real_multilane_dogfood"},
        {"claim_id":"production_readiness"}
    ]);
    let mut review = Vec::new();
    crate::review::round::claim::ceiling::row_authority_errors(
        &repo,
        &ceiling,
        &crate::self_tests::review::claim_ceiling::anchors(),
        &json!({"claim_ceiling_assessment":{
            "supported":[
                {"claim_id":"package_static_fixture_proof"},
                {"claim_id":"detached_review_target_archive_identity"}
            ],
            "unsupported":[
                {"claim_id":"plugins_ui_visibility"},
                {"claim_id":"install_button_success"},
                {"claim_id":"workspace_public_marketplace_publication"},
                {"claim_id":"real_multilane_dogfood"},
                {"claim_id":"production_readiness"},
                {"claim_id":"external_product_ux_improvement"}
            ]
        }}),
        "contract_claim_falsifier",
        &mut review,
    );
    assert!(
        review
            .iter()
            .any(|failure| failure.error == "review_round_claim_ceiling_missing")
    );
    std::fs::remove_dir_all(root).expect("cleanup session_and_review semantic review red");
}
