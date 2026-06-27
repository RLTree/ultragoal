use serde_json::json;

pub(crate) fn anchors() -> crate::review::round::anchor::values::AnchorValues {
    crate::review::round::anchor::values::AnchorValues {
        validator_path: "validator.json".into(),
        review_target_path: "review-target.json".into(),
        archive_path: "archive.json".into(),
        validator_digest: crate::self_tests::boundaries::support::sha('1'),
        review_target_digest: crate::self_tests::boundaries::support::sha('2'),
        archive_digest: crate::self_tests::boundaries::support::sha('3'),
        validator_run_id: "run".into(),
        package_digest: crate::self_tests::boundaries::support::sha('4'),
        source_errors: Vec::new(),
    }
}

pub(crate) fn receipt() -> serde_json::Value {
    json!({"claim_ceiling":{
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
    }})
}

#[test]
fn review_round_claim_ceiling_rejects_substitutes_and_mismatches() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let anchors = anchors();
    let mut out = Vec::new();
    crate::review::round::claim::ceiling::row_authority_errors(
        &root,
        &receipt(),
        &anchors,
        &json!({
            "counterexamples_attempted":[
                {"probe_id":"p","counterexample":"same","evidence":{"path":"missing","digest":crate::self_tests::boundaries::support::sha('a')}},
                {"probe_id":"p","counterexample":"same","evidence":{"path":"missing","digest":crate::self_tests::boundaries::support::sha('a')}},
                {"probe_id":"q","counterexample":"other","evidence":{"path":"missing","digest":crate::self_tests::boundaries::support::sha('a')}}
            ],
            "proof_anchors_checked":[{"digest":crate::self_tests::boundaries::support::sha('1')},{"path":"validator.json"}],
            "required_next_repairs":[],
            "claim_ceiling_assessment":{"supported":[{"claim_id":"package_static_fixture_proof"}],"unsupported":[]}
        }),
        "unknown_persona",
        &mut out,
    );
    let errors = out
        .iter()
        .map(|failure| failure.error.as_str())
        .collect::<Vec<_>>();
    assert!(errors.contains(&"review_round_counterexamples_missing"));
    assert!(errors.contains(&"review_round_claim_ceiling_missing"));
    assert!(errors.contains(&"review_round_report_too_shallow"));

    let mut mismatch = receipt();
    mismatch["claim_ceiling"]["supported"] =
        json!([{"claim_id":"external_product_ux_improvement"}]);
    out.clear();
    crate::review::round::claim::ceiling::row_authority_errors(
        &root,
        &mismatch,
        &anchors,
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
        &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "review_round_claim_ceiling_overclaim")
    );
}

#[test]
fn review_round_claim_ceiling_covers_anchor_shape_duplicates_and_mismatch() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let anchors = anchors();
    let mut out = Vec::new();
    crate::review::round::claim::ceiling::row_authority_errors(
        &root,
        &receipt(),
        &anchors,
        &json!({
            "counterexamples_attempted":[
                {"probe_id":"a","counterexample":"one","evidence":{"path":"missing-a","digest":crate::self_tests::boundaries::support::sha('a')}},
                {"probe_id":"b","counterexample":"two","evidence":{"path":"missing-b","digest":crate::self_tests::boundaries::support::sha('b')}},
                {"probe_id":"c","counterexample":"three","evidence":{"path":"missing-c","digest":crate::self_tests::boundaries::support::sha('c')}}
            ],
            "proof_anchors_checked":[
                {"digest":crate::self_tests::boundaries::support::sha('1')},
                {"path":"validator.json"},
                {"path":"agents/contract-claim-falsifier.md","digest":crate::digest::file(&root.join("agents/contract-claim-falsifier.md")).unwrap()},
                {"path":"custom-agents/harness-contract-claim-falsifier.toml","digest":crate::digest::file(&root.join("custom-agents/harness-contract-claim-falsifier.toml")).unwrap()},
                {"path":"docs/hypercritical-review-law.md","digest":crate::digest::file(&root.join("docs/hypercritical-review-law.md")).unwrap()}
            ],
            "required_next_repairs":[{"evidence":{"path":"missing-repair","digest":crate::self_tests::boundaries::support::sha('d')}}]
        }),
        "contract_claim_falsifier",
        &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "review_round_claim_ceiling_missing")
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "review_round_report_artifact_invalid")
    );

    let mut duplicate = json!({
        "claim_ceiling_assessment":{
            "supported":[
                {"claim_id":"package_static_fixture_proof"},
                {"claim_id":"package_static_fixture_proof"}
            ],
            "unsupported":[{"claim_id":"plugins_ui_visibility"}]
        }
    });
    out.clear();
    crate::review::round::claim::ceiling::row_authority_errors(
        &root,
        &receipt(),
        &anchors,
        &duplicate,
        "contract_claim_falsifier",
        &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "review_round_claim_ceiling_duplicate")
    );

    duplicate["claim_ceiling_assessment"] = json!({
        "supported":[
            {"claim_id":"package_static_fixture_proof"},
            {"claim_id":"detached_review_target_archive_identity"}
        ],
        "unsupported":[{"claim_id":"plugins_ui_visibility"}]
    });
    out.clear();
    crate::review::round::claim::ceiling::row_authority_errors(
        &root,
        &receipt(),
        &anchors,
        &duplicate,
        "contract_claim_falsifier",
        &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "review_round_claim_ceiling_mismatch")
    );

    let mut live_overclaim = receipt();
    live_overclaim["claim_ceiling"]["supported"] =
        json!([{"claim_id":"live_runtime_ui_visibility"}]);
    out.clear();
    crate::review::round::claim::ceiling::row_authority_errors(
        &root,
        &live_overclaim,
        &anchors,
        &duplicate,
        "contract_claim_falsifier",
        &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "review_round_claim_ceiling_overclaim")
    );
}
