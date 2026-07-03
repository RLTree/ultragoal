use serde_json::{Value, json};
use std::path::Path;

pub(super) fn refs(root: &Path, current: &str) -> Vec<Value> {
    vec![
        fit_repo_ref(root, current),
        product_fitness_ref(root, current),
        product_journey_ref(root, current),
    ]
}

fn fit_repo_ref(root: &Path, current: &str) -> Value {
    let stdout = super::ref_for(
        root,
        "validation_artifacts/harness/fit-repo-command.stdout.txt",
        &json!("fit-repo stdout"),
    );
    let stderr = super::ref_for(
        root,
        "validation_artifacts/harness/fit-repo-command.stderr.txt",
        &json!("fit-repo stderr"),
    );
    let mut receipt = json!({
        "schema":"harness-ultragoal.fit-repo-receipt.v1",
        "entrypoint_contract":{"id":"harness-ultragoal:fit-repo","version":"1"},
        "target_revision":{"kind":"package_digest","value":current},
        "plugin_source_path":".",
        "installed_plugin_path":"codex-installed-plugin:harness-ultragoal",
        "cache_package_path":"local-harness-plugins/harness-ultragoal/0.0.0-test",
        "plugin_version":"0.0.0-test",
        "producer_actor_id":"test-parent-session",
        "target_classification":"fresh_repo",
        "runtime_surface_classification":"none",
        "product_surface_classification":"developer_tool",
        "checks":[{"id":"fit_repo","command":"ultragoal fit-repo","exit":0,"stdout":stdout,"stderr":stderr}],
        "claim_ceiling":"withheld_or_blocked",
        "blockers":[],
        "receipt_digest":crate::digest::ZERO
    });
    receipt["receipt_digest"] = json!(crate::audit::fit_repo_receipt::canonical_digest(&receipt));
    receipt_ref(
        root,
        "validation_artifacts/harness/fit-repo-receipt.json",
        &receipt,
    )
}

fn product_fitness_ref(root: &Path, current: &str) -> Value {
    let evidence = evidence_ref(
        root,
        "validation_artifacts/harness/product-fitness-proof.txt",
    );
    let substitutions = [
        "install",
        "first run",
        "smoke",
        "test pass",
        "package publication",
        "reviewer approved",
        "opinion",
        "feature delivered",
        "happy path",
        "fixture",
        "daily driver",
        "single run",
        "entrypoint confusion",
        "hidden power",
    ]
    .into_iter()
    .map(|rejected| json!({"rejected_substitute":rejected,"reason":"same-surface outcome proof required"}))
    .collect::<Vec<_>>();
    let mut receipt = json!({
        "schema":"harness-ultragoal.product-fitness-receipt.v1",
        "receipt_version":"1.0.0",
        "claim":{"id":"CLAIM-001","title":"Product Fitness proof is enforced","repeated_use_claimed":false},
        "target_revision":{"kind":"package_digest","value":current},
        "target_audience":{"name":"Harness operator","evidence":evidence.clone()},
        "job_to_be_done":{"job":"Evaluate product-impacting claims","evidence":evidence.clone()},
        "context_of_use":{"context":"Local strict validation loop","evidence":evidence.clone()},
        "desired_user_outcome":{"outcome":"Unsupported readiness claims are blocked","evidence":evidence.clone()},
        "business_or_mission_outcome":{"outcome":"Prevent false product readiness","evidence":evidence.clone()},
        "critical_journey":{"id":"journey","steps":["classify claim","require receipt"]},
        "first_value_event":{"event":"claim blocked without Product Fitness","evidence":evidence.clone()},
        "continuance_signal":{"required":false,"evidence":evidence.clone()},
        "assumption_tests":[{"id":"assumption","hypothesis":"substitutes are rejected","evidence":evidence.clone()}],
        "user_evidence":[{"audience":"Harness operator","job":"Evaluate product-impacting claims","context":"Local strict validation loop","outcome":"Unsupported readiness claims are blocked","evidence":evidence.clone()}],
        "quality_in_use_metrics":quality_metrics(&evidence),
        "accessibility_gate":{"journey_binding":"journey","evidence":evidence.clone()},
        "cognitive_load_gate":{"burden_assessment":"bounded receipt review","evidence":evidence.clone()},
        "recovery_burden_gate":{"recovery_path":"rerun proof command","evidence":evidence.clone()},
        "proof_surface":{"kind":"package_static_fixture","evidence":evidence.clone()},
        "substitution_rejections":substitutions,
        "claim_ceiling":"package_static_fixture_only",
        "producer_actor_id":"product-fitness-producer",
        "reviewer_actor_id":"product-fitness-reviewer",
        "actor_disjoint":true,
        "generated_at":"2026-06-27T00:00:00Z",
        "receipt_digest":crate::digest::ZERO
    });
    receipt["receipt_digest"] = json!(crate::audit::product::fitness::receipt::canonical_digest(
        &receipt
    ));
    receipt_ref(
        root,
        "validation_artifacts/harness/product-fitness-receipt.json",
        &receipt,
    )
}

fn quality_metrics(evidence: &Value) -> Vec<Value> {
    [
        ("effectiveness", "substitutes fail"),
        ("efficiency", "single receipt path"),
        ("satisfaction", "claim ceiling is explicit"),
        ("freedom_from_risk", "overclaims blocked"),
        ("context_coverage", "source-local validation loop covered"),
    ]
    .into_iter()
    .map(|(dimension, measure)| {
        json!({"dimension":dimension,"measure":measure,"evidence":evidence.clone()})
    })
    .collect()
}

fn product_journey_ref(root: &Path, current: &str) -> Value {
    let fit_repo_receipt = fit_repo_ref(root, current);
    let extra = evidence_ref(
        root,
        "validation_artifacts/harness/product-journey-proof.txt",
    );
    let error = evidence_ref(
        root,
        "validation_artifacts/harness/product-journey-error.txt",
    );
    let receipt = json!({
        "schema":"harness-ultragoal.plugin-product-journey-receipt.v1",
        "status":"pass",
        "target_revision":{"kind":"package_digest","value":current},
        "generated_at":"2026-06-27T00:00:00Z",
        "claim_ceiling":"package_static_fixture_only",
        "journey":[
            "invoke fit-repo",
            "classify target",
            "verify setup",
            "run standards",
            "run coverage",
            "run product fitness",
            "emit receipt",
            "compute claim ceiling",
            "block unsupported claims",
            "record evidence"
        ],
        "evidence":[fit_repo_receipt, extra.clone(), extra.clone()],
        "error_path_evidence":error
    });
    receipt_ref(
        root,
        "validation_artifacts/harness/plugin-product-journey-receipt.json",
        &receipt,
    )
}

fn receipt_ref(root: &Path, rel: &str, value: &Value) -> Value {
    super::ref_for(root, rel, value)
}

fn evidence_ref(root: &Path, rel: &str) -> Value {
    let path = root.join(rel);
    let parent = path.parent().expect("evidence parent");
    std::fs::create_dir_all(parent).expect("evidence parent dir");
    std::fs::write(&path, format!("evidence for {rel}\n")).expect("evidence write");
    json!({"path":rel,"digest":crate::digest::file(&path).expect("evidence digest")})
}
