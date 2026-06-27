use serde_json::{Value, json};
use std::path::Path;

fn write_json_artifact(root: &Path, rel: &str, value: &Value) -> Value {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("artifact parent");
    std::fs::write(&path, serde_json::to_vec(value).expect("json")).expect("artifact write");
    json!({"path":rel,"digest":crate::digest::file(&path).expect("digest")})
}

#[test]
fn target_repo_product_review_rejects_actor_authority_and_payload_mismatch() {
    let root = crate::self_tests::boundaries::support::temp_root("target-product-review");
    assert!(
        crate::target_repo::product::review::review_error(
            &root,
            &json!({"review":{"reviewers":[]}})
        )
        .expect("empty reviewers")
        .contains("at least one reviewer")
    );
    assert!(
        crate::target_repo::product::review::review_error(
            &root,
            &json!({"review":{"reviewers":["author"]}}),
        )
        .expect("banned reviewer")
        .contains("actor-disjoint")
    );
    assert!(
        crate::target_repo::product::review::review_error(
            &root,
            &json!({
                "review":{
                    "reviewers":["Reviewer A"],
                    "method":"four-person",
                    "reviewer_authority":{"actor_disjoint":false}
                }
            }),
        )
        .expect("authority disjoint")
        .contains("authority must be actor-disjoint")
    );

    let evidence = write_json_artifact(
        &root,
        "evidence/review.json",
        &json!({
            "schema":"harness-ultragoal.product-cohesion-review.v1",
            "status":"pass",
            "actor_disjoint":true,
            "authority":"product-review",
            "reviewers":["Reviewer A"],
            "method":"other"
        }),
    );
    let err = crate::target_repo::product::review::review_error(
        &root,
        &json!({
            "review":{
                "reviewers":["Reviewer A"],
                "method":"four-person",
                "reviewer_authority":{
                    "actor_disjoint":true,
                    "authority":"product-review",
                    "evidence":evidence
                }
            }
        }),
    )
    .expect("method mismatch");
    assert!(err.contains("method mismatch"));

    std::fs::remove_dir_all(root).expect("cleanup target product review");
}

#[test]
fn target_repo_product_review_rejects_bad_evidence_payloads() {
    let root = crate::self_tests::boundaries::support::temp_root("target-product-review-payloads");
    std::fs::create_dir_all(&root).expect("target product review root");
    let base_receipt = |evidence: Value| {
        json!({
            "review":{
                "reviewers":["Reviewer A"],
                "method":"four-person",
                "reviewer_authority":{
                    "actor_disjoint":true,
                    "authority":"product-review",
                    "evidence":evidence
                }
            }
        })
    };
    let missing = crate::target_repo::product::review::review_error(
        &root,
        &base_receipt(json!({"path":"missing.json","digest":crate::self_tests::boundaries::support::sha('a')})),
    )
    .expect("missing evidence");
    assert!(missing.contains("artifact missing"));

    for (name, payload, expected) in [
        (
            "schema",
            json!({
                "schema":"wrong",
                "status":"pass",
                "actor_disjoint":true,
                "authority":"product-review",
                "reviewers":["Reviewer A"],
                "method":"four-person"
            }),
            "schema mismatch",
        ),
        (
            "status",
            json!({
                "schema":"harness-ultragoal.product-cohesion-review.v1",
                "status":"fail",
                "actor_disjoint":true,
                "authority":"product-review",
                "reviewers":["Reviewer A"],
                "method":"four-person"
            }),
            "status not pass",
        ),
        (
            "authority",
            json!({
                "schema":"harness-ultragoal.product-cohesion-review.v1",
                "status":"pass",
                "actor_disjoint":true,
                "authority":"other",
                "reviewers":["Reviewer A"],
                "method":"four-person"
            }),
            "authority mismatch",
        ),
        (
            "reviewers",
            json!({
                "schema":"harness-ultragoal.product-cohesion-review.v1",
                "status":"pass",
                "actor_disjoint":true,
                "authority":"product-review",
                "reviewers":["Reviewer B"],
                "method":"four-person"
            }),
            "reviewers mismatch",
        ),
    ] {
        let evidence = write_json_artifact(&root, &format!("evidence/{name}.json"), &payload);
        let err = crate::target_repo::product::review::review_error(&root, &base_receipt(evidence))
            .expect("payload failure");
        assert!(err.contains(expected), "{expected}: {err}");
    }
    std::fs::remove_dir_all(root).expect("cleanup target product review payloads");
}
