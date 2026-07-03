use crate::audit::contract::Failure;
use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn has_fail(failures: &[Failure], error: &str) -> bool {
    failures.iter().any(|failure| failure.error == error)
}

#[test]
fn claim_evidence_reports_live_missing_stale_workspace_and_command_mismatch() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("claim-evidence");
    std::fs::create_dir_all(root.join("artifacts")).expect("artifacts");
    let proof = root.join("artifacts/proof.json");
    std::fs::write(&proof, "{}").expect("proof");
    let digest = crate::digest::file(&proof).expect("digest");
    let cm = json!({"commit":"commit-a","root":"workspace-a"});
    let ready = json!({"commands":[{"id":"cmd","exit":1,"artifact_path":"artifacts/proof.json","artifact_digest":digest}]});
    let claim = json!({
        "id":"CLAIM-EVIDENCE",
        "claim_kind":"feature_completion",
        "status":"proven_static",
        "allowed_evidence_surfaces":["source"],
        "evidence":[
            {
                "id":"ev",
                "kind":"live_beneficial_e2e",
                "surface":"fixture",
                "path":"fixtures/fake-proof.json",
                "digest":crate::digest::ZERO,
                "commit":"commit-b",
                "workspace":"workspace-b",
                "produced_by_command_id":"missing",
                "freshness":{"verdict":"stale"},
                "live_beneficial_task":{
                    "real_input_path":"fixture/input.json",
                    "output_artifact_path":"mock/output.json"
                }
            },
            {
                "id":"ev-command",
                "kind":"runtime_receipt",
                "surface":"source",
                "path":"artifacts/proof.json",
                "digest":digest,
                "commit":"commit-a",
                "workspace":"workspace-a",
                "produced_by_command_id":"cmd",
                "freshness":{"verdict":"current"}
            }
        ]
    });
    let mut failures = Vec::new();
    crate::claim_semantics::claim::evidence::evidence_checks(
        &claim,
        &cm,
        &ready,
        &root,
        &mut failures,
    );
    for expected in [
        "proof_surface_substitution",
        "evidence_stale_or_invalidated",
        "evidence_wrong_workspace",
        "evidence_digest_missing_or_mismatched",
        "simulated_evidence_used_for_live_claim",
        "command_receipt_missing_or_failed",
    ] {
        assert!(has_fail(&failures, expected), "{expected}: {failures:?}");
    }

    failures.clear();
    crate::claim_semantics::claim::evidence::live_e2e_check(
        &json!({"id":"CLAIM-NO-LIVE","evidence":[]}),
        &ready,
        &root,
        &mut failures,
    );
    assert!(has_fail(&failures, "live_beneficial_e2e_missing"));
    std::fs::remove_dir_all(root).expect("cleanup claim evidence");
}

#[test]
fn claim_evidence_reaches_command_artifact_integrity_branches() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "claim-evidence-command-artifacts",
    );
    std::fs::create_dir_all(root.join("artifacts")).expect("artifacts");
    let proof = root.join("artifacts/proof.json");
    std::fs::write(&proof, "{}").expect("proof");
    let digest = crate::digest::file(&proof).expect("digest");
    let cm = json!({"commit":"commit-a","root":"workspace-a"});
    let ready = json!({"commands":[
        {"id":"mismatch","exit":0,"artifact_path":"artifacts/missing.json","artifact_digest":digest},
        {"id":"escape","exit":0,"artifact_path":"../escape.json","artifact_digest":digest}
    ]});
    let claim = json!({
        "id":"CLAIM-COMMAND-ARTIFACTS",
        "claim_kind":"feature_completion",
        "status":"proven_static",
        "allowed_evidence_surfaces":["source"],
        "evidence":[
            {
                "id":"ev-mismatch",
                "kind":"runtime_receipt",
                "surface":"source",
                "path":"artifacts/proof.json",
                "digest":digest,
                "commit":"commit-a",
                "workspace":"workspace-a",
                "produced_by_command_id":"mismatch",
                "freshness":{"verdict":"current"}
            },
            {
                "id":"ev-escape",
                "kind":"runtime_receipt",
                "surface":"source",
                "path":"artifacts/proof.json",
                "digest":digest,
                "commit":"commit-a",
                "workspace":"workspace-a",
                "produced_by_command_id":"escape",
                "freshness":{"verdict":"current"}
            }
        ]
    });
    let mut failures = Vec::new();
    crate::claim_semantics::claim::evidence::evidence_checks(
        &claim,
        &cm,
        &ready,
        &root,
        &mut failures,
    );
    let digest_failures = failures
        .iter()
        .filter(|failure| failure.error == "evidence_digest_missing_or_mismatched")
        .count();
    assert!(digest_failures >= 2, "{failures:?}");
    std::fs::remove_dir_all(root).expect("cleanup claim evidence command artifacts");
}

#[test]
fn coverage_policy_rejects_missing_malformed_and_unresolvable_receipts() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-policy");
    std::fs::create_dir_all(root.join("receipts")).expect("receipts");
    let mut failures = Vec::new();
    crate::claim_semantics::coverage::policy::check(
        &json!({"id":"COV-REVIEWER","title":"Coverage complete because reviewer approved"}),
        &root,
        &mut failures,
    );
    assert!(has_fail(
        &failures,
        "coverage_reviewer_signoff_substitution"
    ));

    failures.clear();
    crate::claim_semantics::coverage::policy::check(
        &json!({
            "id":"COV-ESCAPE",
            "title":"Coverage complete",
            "evidence":[{"id":"cov","kind":"coverage_receipt","path":"../escape.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('1')}]
        }),
        &root,
        &mut failures,
    );
    assert!(has_fail(&failures, "coverage_receipt_missing"));

    write_json(&root.join("receipts/bad.json"), &json!({"not":"coverage"}));
    std::fs::write(root.join("receipts/malformed.json"), "{").expect("malformed");
    failures.clear();
    crate::claim_semantics::coverage::policy::check(
        &json!({
            "id":"COV-BAD",
            "title":"Coverage complete",
            "evidence":[
                {"id":"bad","kind":"coverage_receipt","path":"receipts/bad.json","digest":crate::digest::file(&root.join("receipts/bad.json")).expect("digest"),"freshness":{"verdict":"stale"}},
                {"id":"malformed","kind":"coverage_receipt","path":"receipts/malformed.json","digest":crate::digest::file(&root.join("receipts/malformed.json")).expect("digest")}
            ]
        }),
        &root,
        &mut failures,
    );
    assert!(has_fail(&failures, "coverage_receipt_stale"));
    assert!(has_fail(&failures, "coverage_receipt_malformed"));
    std::fs::remove_dir_all(root).expect("cleanup coverage policy");
}

#[test]
fn coverage_policy_rejects_ratchet_completion_and_missing_behavior_dimensions() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-policy-dimensions");
    std::fs::create_dir_all(root.join("receipts")).expect("receipts");
    write_json(
        &root.join("receipts/ratchet.json"),
        &json!({
            "schema":"harness-ultragoal.coverage-receipt.v1",
            "claim_id":"COV-DIMS",
            "command":"coverage",
            "tool":"llvm-cov",
            "coverage":{"percent":99.0,"policy":"ratchet_floor"},
            "uncovered_records":[],
            "target_paths":["src/lib.rs"],
            "measured_dimensions":["line"]
        }),
    );
    let mut failures = Vec::new();
    crate::claim_semantics::coverage::policy::check(
        &json!({
            "id":"COV-DIMS",
            "title":"Coverage 100 ready for source code branch function region user-facing artifact",
            "description":"The command route handler, UI control surface, schema receipt, and archive are complete.",
            "evidence":[{"id":"cov","kind":"coverage_receipt","path":"receipts/ratchet.json","freshness":{"verdict":"current"}}]
        }),
        &root,
        &mut failures,
    );
    for expected in [
        "coverage_ratchet_missing_owner",
        "coverage_ratchet_missing_debt",
        "coverage_claim_uncovered_code",
        "coverage_ratchet_presented_as_complete",
        "coverage_required_dimension_missing",
    ] {
        assert!(has_fail(&failures, expected), "{expected}: {failures:?}");
    }
    let dimensions = failures
        .iter()
        .filter(|failure| failure.error == "coverage_required_dimension_missing")
        .map(|failure| failure.detail.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for required in ["artifact", "branch", "function", "region", "ui_state"] {
        assert!(dimensions.contains(required), "{required}: {dimensions:?}");
    }
    std::fs::remove_dir_all(root).expect("cleanup coverage dimensions");
}
