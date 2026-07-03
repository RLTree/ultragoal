use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn has(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}

fn gardener_receipt(action: &str) -> Value {
    json!({
        "schema":"harness-ultragoal.standards-gardening-receipt.v1",
        "status":"pass",
        "generated_at":"2026-06-26T00:00:00Z",
        "candidate_digest":crate::self_tests::boundaries::workspace_fixtures::sha('c'),
        "trigger_signal":{
            "signal_id":"sig",
            "severity":"moderate",
            "signal_kind":"standards_entropy",
            "summary":"Repeated standards drift",
            "source":"session-log"
        },
        "decision":{"accepted":true,"action":action,"rationale":"Promote to deterministic guard"},
        "changed_artifacts":[{"path":"docs/law.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('1')}],
        "safeguards":{"deterministic_first":true,"no_hook_by_default":true},
        "claim_ceiling":"package_static_fixture_only"
    })
}

#[test]
fn source_obligation_and_standards_gardener_edges_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("obligation-gardener");
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    assert_eq!(
        crate::audit::source_obligations::value_failures(&json!({})),
        vec!["source_obligation_matrix_missing_rows"]
    );
    let obligations = json!({"obligations":[{
        "id":"derived-authority-recomputation",
        "enforcement_disposition":"deterministic",
        "missing_validation_fixture_or_receipt":"validator receipt",
        "claim_ceiling_impact":"blocks"
    }]});
    let obligation_failures = crate::audit::source_obligations::value_failures(&obligations);
    assert!(has(
        &obligation_failures,
        "derived-authority-recomputation: source_obligation_missing_canonical"
    ));
    let gap_failures = crate::audit::source_obligations::value_failures(&json!({"obligations":[{
        "id":"custom-law",
        "enforcement_disposition":"deterministic"
    }]}));
    assert!(has(
        &gap_failures,
        "custom-law: source_obligation_missing_gap_field"
    ));

    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[]}),
    );
    assert!(crate::audit::standards_gardening::failures(&root, &store).is_empty());
    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[{
            "id":"standards-gardener-promotion",
            "standards_gardening_receipt_path": 7
        }]}),
    );
    assert!(crate::audit::standards_gardening::failures(&root, &store).is_empty());
    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[{
            "id":"standards-gardener-promotion",
            "standards_gardening_receipt_path":"missing.json"
        }]}),
    );
    assert!(has(
        &crate::audit::standards_gardening::failures(&root, &store),
        "standards_gardener_receipt_artifact_invalid"
    ));
    assert_eq!(
        crate::audit::standards_gardening::receipt_failures(&store, &json!({})),
        vec!["standards_gardener_receipt_schema_invalid"]
    );
    std::fs::remove_dir_all(root).expect("cleanup obligation gardener");
}

#[test]
fn standards_gardener_receipts_cover_semantic_and_root_edges() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("standards-gardener-edges");
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    let mut low = gardener_receipt("validator_check");
    low["trigger_signal"]["severity"] = json!("low");
    assert_eq!(
        crate::audit::standards_gardening::receipt_failures(&store, &low),
        vec!["standards_gardener_receipt_semantic_invalid"]
    );
    assert_eq!(
        crate::audit::standards_gardening::receipt_failures(&store, &gardener_receipt("backlog")),
        vec!["standards_gardener_receipt_semantic_invalid"]
    );
    assert_eq!(
        crate::audit::standards_gardening::receipt_failures(&store, &gardener_receipt("hook")),
        vec!["standards_gardener_receipt_semantic_invalid"]
    );
    let mut hook = gardener_receipt("hook");
    hook["safeguards"]["hook_justification"] = json!("Host supports deterministic preflight hook");
    assert!(crate::audit::standards_gardening::receipt_failures(&store, &hook).is_empty());

    std::fs::create_dir_all(root.join("docs")).expect("docs");
    write_json(
        &root.join("docs/law.json"),
        &json!({"generated_at":"2026-06-26T00:10:00Z"}),
    );
    let digest = crate::digest::file(&root.join("docs/law.json")).expect("law digest");
    let mut receipt = gardener_receipt("validator_check");
    receipt["changed_artifacts"][0]["digest"] = json!(digest);
    let root_failures = crate::audit::standards_gardening::receipt_root_failures(&root, &receipt);
    assert!(has(
        &root_failures,
        "standards_gardener_changed_artifact_after_receipt"
    ));
    receipt["changed_artifacts"][0]["digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('0'));
    let root_failures = crate::audit::standards_gardening::receipt_root_failures(&root, &receipt);
    assert!(has(
        &root_failures,
        "standards_gardener_changed_artifact_digest_mismatch"
    ));
    receipt["changed_artifacts"][0]["path"] = json!("docs/missing.json");
    let root_failures = crate::audit::standards_gardening::receipt_root_failures(&root, &receipt);
    assert!(has(
        &root_failures,
        "standards_gardener_changed_artifact_missing"
    ));
    receipt["changed_artifacts"][0]["path"] =
        json!("validation_artifacts/cli/update-goal-eligibility.json");
    let root_failures = crate::audit::standards_gardening::receipt_root_failures(&root, &receipt);
    assert!(has(
        &root_failures,
        "standards_gardener_runtime_receipt_artifact"
    ));
    std::fs::remove_dir_all(root).expect("cleanup standards gardener edges");
}

#[test]
fn source_obligation_rows_reject_weak_dispositions_and_missing_law_tokens() {
    let matrix = json!({"obligations":[
        {
            "id":"missing-disposition",
            "enforcement_disposition":"",
            "missing_validation_fixture_or_receipt":"validator red fixture",
            "claim_ceiling_impact":"blocks"
        },
        {
            "id":"weak-human-audit",
            "enforcement_disposition":"deterministic_with_human_audit",
            "missing_validation_fixture_or_receipt":"validator red fixture",
            "claim_ceiling_impact":"blocks"
        },
        {
            "id":"authority-source-binding",
            "enforcement_disposition":"deterministic authority receipt",
            "missing_validation_fixture_or_receipt":"validator receipt",
            "claim_ceiling_impact":"blocks"
        },
        {
            "id":"source-installed-cache-alignment",
            "enforcement_disposition":"deterministic source installed validator",
            "missing_validation_fixture_or_receipt":"validator receipt",
            "claim_ceiling_impact":"blocks"
        },
        {
            "id":"namespace-progressive-disclosure",
            "enforcement_disposition":"deterministic namespace progressive validator",
            "missing_validation_fixture_or_receipt":"valid fixture and receipt",
            "claim_ceiling_impact":"blocks"
        },
        {
            "id":"derived-authority-recomputation",
            "enforcement_disposition":"deterministic canonical recomputation digest validator",
            "missing_validation_fixture_or_receipt":"red fixture and receipt",
            "claim_ceiling_impact":"blocks"
        }
    ]});
    let failures = crate::audit::source_obligations::value_failures(&matrix);
    for expected in [
        "missing-disposition: source_obligation_missing_disposition",
        "weak-human-audit: source_obligation_weak_disposition:deterministic_with_human_audit",
        "authority-source-binding: source_obligation_missing_fallback",
        "source-installed-cache-alignment: source_obligation_missing_cache",
        "namespace-progressive-disclosure: source_obligation_missing_red",
    ] {
        assert!(has(&failures, expected), "{expected}: {failures:?}");
    }
    assert!(
        !failures
            .iter()
            .any(|failure| failure.starts_with("derived-authority-recomputation:")),
        "{failures:?}"
    );

    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("source-obligation-root");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    write_json(&root.join("docs/source-obligation-matrix.json"), &matrix);
    let root_failures = crate::audit::source_obligations::failures(&root);
    assert!(has(
        &root_failures,
        "missing-disposition: source_obligation_missing_disposition"
    ));
    std::fs::remove_dir_all(root).expect("cleanup source obligation root");
}
