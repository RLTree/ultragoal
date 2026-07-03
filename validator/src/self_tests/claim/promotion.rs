use crate::audit::contract::Failure;
use serde_json::{Value, json};

fn claim(id: &str, text: &str, semantic: Value, evidence: Value) -> Value {
    json!({
        "id": id,
        "title": text,
        "status": "pass",
        "claim_ceiling_effect": "included",
        "semantic_text_classification": semantic,
        "evidence": evidence
    })
}

fn promo_receipt(claim_id: &str, target: &str, package: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.promotion-receipt.v1",
        "status": "pass",
        "promotion_target": target,
        "claim_id": claim_id,
        "validator_receipt": {"path":"v.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('1'),"run_id":"run","package_digest":package},
        "review_target": {"path":"r.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('2'),"review_target_digest":crate::self_tests::boundaries::workspace_fixtures::sha('3'),"package_digest":package},
        "candidate_archive": {"path":"a.zip","digest":crate::self_tests::boundaries::workspace_fixtures::sha('4'),"archive_purpose":"candidate_review_anchor","package_digest":package},
        "sign_off_review": {"path":"s.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('5'),"status":"pass","review_stage":"sign_off","validator_run_id":"run","review_target_digest":crate::self_tests::boundaries::workspace_fixtures::sha('3'),"archive_digest":crate::self_tests::boundaries::workspace_fixtures::sha('4')},
        "decision": {"approved": true, "promoted_at": "2026-06-25T00:00:00Z", "approver_actor_id": "approver"}
    })
}

#[test]
fn promotion_receipts_cover_valid_invalid_and_target_matching() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let temp = crate::self_tests::boundaries::workspace_fixtures::temp_root("promotion-name");
    let unique = temp
        .file_name()
        .and_then(|name| name.to_str())
        .expect("temp name");
    let rel = format!("validation_artifacts/{unique}.promotion.json");
    let path = root.join(&rel);
    let package = crate::self_tests::boundaries::workspace_fixtures::sha('a');
    std::fs::write(
        &path,
        serde_json::to_vec(&promo_receipt("CLAIM-1", "install_visibility", &package))
            .expect("promotion json"),
    )
    .expect("promotion");

    let ev = json!([{
        "id": "PROMO-1",
        "kind": "promotion_receipt",
        "surface": "external",
        "path": rel,
        "digest": crate::digest::file(&path).expect("digest")
    }]);
    let semantic = json!({"install_visibility_claim": true});
    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::promotion_receipt::check(
        &claim("CLAIM-1", "plugin installed visibly", semantic, ev),
        &root,
        &mut out,
    );
    assert!(out.is_empty(), "{out:?}");

    let bad_ev =
        json!([{"id":"P2","kind":"promotion_receipt","surface":"static","path":"missing.json"}]);
    crate::claim_semantics::promotion_receipt::check(
        &claim(
            "CLAIM-2",
            "plugin marketplace publication",
            json!({"publication_claim": true}),
            bad_ev,
        ),
        &root,
        &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "promotion_receipt_invalid")
    );

    crate::claim_semantics::promotion_receipt::check(
        &claim(
            "CLAIM-3",
            "published externally",
            json!({"publication_claim": true}),
            json!([]),
        ),
        &root,
        &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "promotion_receipt_required")
    );
    out.clear();
    let bad_cases = [
        ("malformed", "{".to_string()),
        ("schema", serde_json::to_string(&json!({"schema":"wrong"})).expect("json")),
        (
            "claim",
            serde_json::to_string(&promo_receipt("OTHER", "install_visibility", &package))
                .expect("json"),
        ),
        (
            "target",
            serde_json::to_string(&promo_receipt("CLAIM-4", "publication", &package))
                .expect("json"),
        ),
        (
            "upload-target",
            serde_json::to_string(&promo_receipt("CLAIM-4", "upload_distribution", &package))
                .expect("json"),
        ),
        (
            "unknown-target",
            serde_json::to_string(&promo_receipt("CLAIM-4", "side_loaded", &package))
                .expect("json"),
        ),
        (
            "anchor",
            serde_json::to_string(&json!({
                "schema": "harness-ultragoal.promotion-receipt.v1",
                "status": "pass",
                "promotion_target": "install_visibility",
                "claim_id": "CLAIM-4",
                "validator_receipt": {"run_id":"run","package_digest":package},
                "review_target": {"review_target_digest":crate::self_tests::boundaries::workspace_fixtures::sha('3'),"package_digest":package},
                "candidate_archive": {"digest":crate::self_tests::boundaries::workspace_fixtures::sha('4'),"package_digest":package},
                "sign_off_review": {"status":"pass","review_stage":"sign_off","validator_run_id":"different","review_target_digest":crate::self_tests::boundaries::workspace_fixtures::sha('3'),"archive_digest":crate::self_tests::boundaries::workspace_fixtures::sha('4')},
                "decision": {"approved": true, "promoted_at": "2026-06-25T00:00:00Z", "approver_actor_id": "approver"}
            }))
            .expect("json"),
        ),
    ];
    for (name, body) in bad_cases {
        let rel = format!("validation_artifacts/{unique}.{name}.promotion.json");
        let path = root.join(&rel);
        std::fs::write(&path, body).expect("bad promotion");
        let evidence = json!([{
            "id": format!("PROMO-{name}"),
            "kind": "promotion_receipt",
            "surface": "external",
            "path": rel,
            "digest": crate::digest::file(&path).expect("bad digest")
        }]);
        crate::claim_semantics::promotion_receipt::check(
            &claim(
                "CLAIM-4",
                "plugin installed visibly",
                json!({"install_visibility_claim": true}),
                evidence,
            ),
            &root,
            &mut out,
        );
        std::fs::remove_file(path).expect("cleanup bad promotion");
    }
    assert_eq!(
        out.iter()
            .filter(|failure| failure.error == "promotion_receipt_invalid")
            .count(),
        7,
        "{out:?}"
    );
    std::fs::remove_file(path).expect("cleanup");
}

#[test]
fn promotion_target_matching_rejects_unknown_targets_after_schema_layer() {
    assert!(
        !crate::claim_semantics::promotion_receipt::target_matches_claim(
            &claim(
                "CLAIM-X",
                "published",
                json!({"install_visibility_claim": true, "publication_claim": true, "distribution_claim": true}),
                json!([])
            ),
            &json!({"promotion_target": "side_loaded"})
        )
    );
    assert!(
        crate::claim_semantics::promotion_receipt::target_matches_claim(
            &claim(
                "CLAIM-I",
                "installed visibly",
                json!({"install_visibility_claim": true}),
                json!([])
            ),
            &json!({"promotion_target": "install_visibility"})
        )
    );
    assert!(
        crate::claim_semantics::promotion_receipt::target_matches_claim(
            &claim(
                "CLAIM-U",
                "uploaded",
                json!({"distribution_claim": true}),
                json!([])
            ),
            &json!({"promotion_target": "upload_distribution"})
        )
    );
}
