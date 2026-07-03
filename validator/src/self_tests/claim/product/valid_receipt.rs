use crate::audit::contract::Failure;
use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn proof_ref(path: &str, digest: &str) -> Value {
    json!({"path": path, "digest": digest})
}

fn substitution_rejections() -> Value {
    json!([
        {"rejected_substitute":"install", "reason":"same-surface user outcome required"},
        {"rejected_substitute":"first run", "reason":"continuance evidence is separate"},
        {"rejected_substitute":"smoke", "reason":"smoke tests do not prove user value"},
        {"rejected_substitute":"test pass", "reason":"test pass is not quality in use"},
        {"rejected_substitute":"package publication", "reason":"publication is not adoption"},
        {"rejected_substitute":"reviewer approved", "reason":"review is not user evidence"},
        {"rejected_substitute":"opinion", "reason":"opinion cannot replace evidence"},
        {"rejected_substitute":"feature delivered", "reason":"output is not outcome"},
        {"rejected_substitute":"happy path", "reason":"failure paths are required"},
        {"rejected_substitute":"fixture", "reason":"fixture is not real use"},
        {"rejected_substitute":"daily driver", "reason":"daily-driver claims need continuance"},
        {"rejected_substitute":"single run", "reason":"single run is not retention"},
        {"rejected_substitute":"entrypoint confusion", "reason":"entrypoint clarity is required"},
        {"rejected_substitute":"hidden power", "reason":"surface discoverability is required"}
    ])
}

fn current_receipt(root: &Path, claim_id: &str) -> Value {
    std::fs::create_dir_all(root).expect("product fitness root");
    std::fs::write(root.join("proof.txt"), "same surface product proof\n").expect("proof");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["proof.txt"]}),
    );
    let package_digest = crate::package::inventory::package_digest(root).expect("package digest");
    let proof_digest = crate::digest::file(&root.join("proof.txt")).expect("proof digest");
    let proof = proof_ref("proof.txt", &proof_digest);
    let mut receipt = json!({
        "schema":"harness-ultragoal.product-fitness-receipt.v1",
        "claim":{"id":claim_id, "repeated_use_claimed":false},
        "target_revision":{"kind":"package_digest", "value":package_digest},
        "target_audience":{"name":"Harness operator maintaining a plugin proposal"},
        "job_to_be_done":{"job":"Decide whether a plugin proposal is materially reviewable"},
        "context_of_use":{"context":"Local source repair and strict validation loop"},
        "desired_user_outcome":{"outcome":"Know unsupported readiness claims are blocked"},
        "business_or_mission_outcome":{"outcome":"Prevent false completion of law packages"},
        "critical_journey":{"id":"review-readiness-loop"},
        "proof_surface":{"kind":"same_surface_quality_in_use"},
        "first_value_event":{"evidence":proof.clone()},
        "assumption_tests":[{"id":"assumption-1", "evidence":proof.clone()}],
        "user_evidence":[{"id":"use-1", "evidence":proof.clone()}],
        "quality_in_use_metrics":[
            {"dimension":"effectiveness", "evidence":proof.clone()},
            {"dimension":"efficiency", "evidence":proof.clone()},
            {"dimension":"satisfaction", "evidence":proof.clone()},
            {"dimension":"freedom_from_risk", "evidence":proof.clone()},
            {"dimension":"context_coverage", "evidence":proof.clone()}
        ],
        "accessibility_gate":{"journey_binding":"review-readiness-loop"},
        "cognitive_load_gate":{"burden_assessment":"bounded evidence paths reduce review load"},
        "recovery_burden_gate":{"recovery_path":"rerun the same source validator command"},
        "substitution_rejections":substitution_rejections(),
        "claim_ceiling":"same_surface_product_fitness_only",
        "producer_actor_id":"product-fitness-producer",
        "reviewer_actor_id":"product-fitness-reviewer",
        "receipt_digest":crate::digest::ZERO
    });
    receipt["receipt_digest"] = json!(crate::audit::product::fitness::receipt::canonical_digest(
        &receipt
    ));
    receipt
}

#[test]
fn product_fitness_accepts_current_typed_receipt_and_rejects_malformed_receipts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "product-fitness-valid-receipt",
    );
    let receipt = current_receipt(&root, "PF-GREEN");
    let receipt_path = root.join("receipts/product-fitness.json");
    write_json(&receipt_path, &receipt);
    let mut claim = json!({
        "id":"PF-GREEN",
        "title":"Product readiness quality in use for operator ready source repair",
        "claim_ceiling_effect":"included",
        "evidence":[{
            "kind":"product::fitness::receipt",
            "surface":"product::fitness",
            "path":"receipts/product-fitness.json",
            "digest":crate::digest::file(&receipt_path).expect("receipt digest")
        }]
    });
    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::product::fitness::check(&claim, &root, &mut out);
    assert!(out.is_empty(), "{out:?}");

    claim["evidence"][0]["path"] = json!("receipts/missing.json");
    claim["evidence"][0]["digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('1'));
    crate::claim_semantics::product::fitness::check(&claim, &root, &mut out);
    assert!(
        out.iter()
            .any(|failure| failure.error == "product_fitness_receipt_missing")
    );

    out.clear();
    let bad_path = root.join("receipts/malformed.json");
    std::fs::write(&bad_path, b"{").expect("malformed receipt");
    claim["evidence"][0]["path"] = json!("receipts/malformed.json");
    claim["evidence"][0]["digest"] = json!(crate::digest::file(&bad_path).expect("bad digest"));
    crate::claim_semantics::product::fitness::check(&claim, &root, &mut out);
    assert!(
        out.iter()
            .any(|failure| failure.error == "product_fitness_receipt_malformed")
    );
    std::fs::remove_dir_all(root).expect("cleanup product fitness valid receipt");
}
