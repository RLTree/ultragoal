use crate::audit::contract::Failure;
use serde_json::json;

fn has_fail(failures: &[Failure], error: &str) -> bool {
    failures.iter().any(|failure| failure.error == error)
}

#[test]
fn live_e2e_and_included_claims_require_real_current_command_evidence() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("claim-evidence-boundary");
    std::fs::create_dir_all(root.join("artifacts")).expect("artifacts");
    let proof = root.join("artifacts/live.json");
    std::fs::write(&proof, br#"{"ok":true}"#).expect("proof");
    let digest = crate::digest::file(&proof).expect("digest");
    let cm = json!({"commit":"rev-a","root":"workspace-a"});
    let ready = json!({"commands":[
        {"id":"ok","exit":0,"artifact_path":"artifacts/live.json","artifact_digest":digest}
    ]});

    let included = json!({
        "id":"CLAIM-INCLUDED",
        "claim_kind":"internal_enforcement",
        "status":"draft",
        "claim_ceiling_effect":"included",
        "allowed_evidence_surfaces":["source"],
        "evidence":[{
            "id":"ev-included",
            "kind":"runtime_receipt",
            "surface":"source",
            "path":"artifacts/live.json",
            "digest":digest,
            "commit":"rev-a",
            "workspace":"workspace-a",
            "produced_by_command_id":"ok",
            "freshness":{"verdict":"current"}
        }]
    });
    let mut failures = Vec::new();
    crate::claim_semantics::claim::evidence::evidence_checks(
        &included,
        &cm,
        &ready,
        &root,
        &mut failures,
    );
    assert!(failures.is_empty(), "{failures:?}");

    let live = json!({
        "id":"CLAIM-LIVE",
        "evidence":[{
            "id":"live-fake",
            "kind":"live_beneficial_e2e",
            "surface":"source",
            "path":"artifacts/live.json",
            "digest":crate::digest::ZERO,
            "commit":"rev-a",
            "workspace":"workspace-a",
            "produced_by_command_id":"missing",
            "freshness":{"verdict":"current"},
            "live_beneficial_task":{
                "real_input_path":"fixture/input.json",
                "output_artifact_path":"mock/output.json"
            }
        }]
    });
    failures.clear();
    crate::claim_semantics::claim::evidence::live_e2e_check(&live, &ready, &root, &mut failures);
    for expected in [
        "live_beneficial_e2e_not_live",
        "evidence_digest_missing_or_mismatched",
        "command_receipt_missing_or_failed",
    ] {
        assert!(has_fail(&failures, expected), "{expected}: {failures:?}");
    }
    std::fs::remove_dir_all(root).expect("cleanup claim evidence boundary");
}
