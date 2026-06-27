use crate::audit::contract::Failure;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

fn errors(out: &[Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn backlog_policy_reports_duplicate_missing_mismatch_and_fake_attempts() {
    let claim_a = json!({"id":"C1","backlog_row_id":"row1"});
    let claim_b = json!({"id":"C2","backlog_row_id":"row2"});
    let mut claims = BTreeMap::new();
    claims.insert("C1".to_string(), &claim_a);
    claims.insert("C2".to_string(), &claim_b);
    let backlog = json!({"rows":[
        {"id":"row1","claim_id":"OTHER","attempts":[]},
        {"id":"row2","claim_id":"MISSING","attempts":[{"id":"attempt","action":"Generic attempted workaround"}]},
        {"id":"row3","claim_id":"MISSING","attempts":[]}
    ]});
    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::backlog_policy::check_backlog(&backlog, &claims, &mut out);
    let got = errors(&out);
    for expected in [
        "backlog_row_claim_duplicate_or_missing",
        "backlog_row_claim_mismatch",
        "backlog_row_claim_missing",
        "blocker_without_attempt_evidence",
        "fake_generic_attempt_satisfies_blocker",
    ] {
        assert!(got.contains(&expected), "{expected}: {got:?}");
    }
}

#[test]
fn dogfood_receipts_reject_missing_invalid_and_semantically_weak_proof() {
    let root = crate::self_tests::boundaries::support::temp_root("dogfood-claims");
    let claim = json!({
        "id":"DOG",
        "title":"Real multi-lane dogfood is complete",
        "description":"The multi lane dogfood proof is ready.",
        "status":"pass",
        "claim_ceiling_effect":"included"
    });
    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::dogfood_receipt::check(&claim, &root, &mut out);
    assert!(errors(&out).contains(&"dogfood_receipt_required"));

    let mut with_bad_surface = claim.clone();
    with_bad_surface["evidence"] =
        json!([{"id":"dog","kind":"dogfood_receipt","surface":"fixture"}]);
    out.clear();
    crate::claim_semantics::dogfood_receipt::check(&with_bad_surface, &root, &mut out);
    assert!(errors(&out).contains(&"dogfood_receipt_invalid"));

    let mut with_bad_ref = claim.clone();
    with_bad_ref["evidence"] = json!([{
        "id":"dog-bad-ref",
        "kind":"dogfood_receipt",
        "surface":"root_integration",
        "path":"../dogfood.json",
        "digest":crate::self_tests::boundaries::support::sha('1')
    }]);
    out.clear();
    crate::claim_semantics::dogfood_receipt::check(&with_bad_ref, &root, &mut out);
    assert!(errors(&out).contains(&"dogfood_receipt_invalid"));

    let receipt_path = root.join("receipts/dogfood.json");
    write_json(&receipt_path, &json!({"schema":"wrong","claim_id":"DOG"}));
    let mut with_bad_schema = claim;
    with_bad_schema["evidence"] = json!([{
        "id":"dog",
        "kind":"dogfood_receipt",
        "surface":"root_integration",
        "path":"receipts/dogfood.json",
        "digest":crate::digest::file(&receipt_path).expect("digest")
    }]);
    out.clear();
    crate::claim_semantics::dogfood_receipt::check(&with_bad_schema, &root, &mut out);
    assert!(errors(&out).contains(&"dogfood_receipt_invalid"));

    let malformed_path = root.join("receipts/dogfood-malformed.json");
    std::fs::write(&malformed_path, "{").expect("malformed dogfood receipt");
    let mut with_malformed = with_bad_schema.clone();
    with_malformed["evidence"] = json!([{
        "id":"dog-malformed",
        "kind":"dogfood_receipt",
        "surface":"root_integration",
        "path":"receipts/dogfood-malformed.json",
        "digest":crate::digest::file(&malformed_path).expect("malformed digest")
    }]);
    out.clear();
    crate::claim_semantics::dogfood_receipt::check(&with_malformed, &root, &mut out);
    assert!(errors(&out).contains(&"dogfood_receipt_invalid"));

    let weak = json!({
        "lanes":[{"lane_id":"one","workspace":"same","macro_lane":false,"cleanup":{"status":"stale","stale_worktree_present":true},"lane_owed_repair":{"status":"open"}}],
        "root_owed_follow_up":{"status":"open","final_manifest_clean":false},
        "external_review":{"status":"missing"}
    });
    assert!(crate::claim_semantics::dogfood_receipt::dogfood_semantics_invalid(&weak));
    let strong = json!({
        "lanes":[
            {"lane_id":"one","workspace":"w1","macro_lane":true,"cleanup":{"status":"cleaned","stale_worktree_present":false},"lane_owed_repair":{"status":"closed"}},
            {"lane_id":"two","workspace":"w2","macro_lane":true,"cleanup":{"status":"cleaned","stale_worktree_present":false},"lane_owed_repair":{"status":"not_applicable"}}
        ],
        "root_owed_follow_up":{"status":"closed","final_manifest_clean":true},
        "external_review":{"status":"pass"}
    });
    assert!(!crate::claim_semantics::dogfood_receipt::dogfood_semantics_invalid(&strong));
    std::fs::remove_dir_all(root).expect("cleanup dogfood");
}

#[test]
fn lane_dependency_reports_missing_upstream_bad_ready_and_release_gaps() {
    let lane = json!({"id":"down","dependency_release":{}});
    let dep = json!({"upstream_lane_id":"up","claim_id":"C","required_status":"merged"});
    let root_phases = json!({"post_merge_integration_gate":{"status":"pass"}});
    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::lane::dependency::check(
        &lane,
        &dep,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &root_phases,
        &json!({}),
        &crate::self_tests::boundaries::support::repo_root(),
        &mut out,
    );
    assert!(errors(&out).contains(&"dependency_claim_not_proven"));

    let upstream = json!({
        "id":"up",
        "status":"merged",
        "claim_ids":["C"],
        "ready_receipt":{"path":"ready.json","digest":crate::self_tests::boundaries::support::sha('1')}
    });
    let mut lanes = BTreeMap::new();
    lanes.insert("up".to_string(), &upstream);
    let dep = json!({
        "upstream_lane_id":"up",
        "claim_id":"C",
        "required_status":"merged",
        "validated_status":"merged",
        "evidence_digest":crate::self_tests::boundaries::support::sha('1'),
        "upstream_ready_receipt":{"path":"ready.json","digest":crate::self_tests::boundaries::support::sha('1')}
    });
    out.clear();
    crate::claim_semantics::lane::dependency::check(
        &lane,
        &dep,
        &lanes,
        &BTreeMap::new(),
        &root_phases,
        &json!({}),
        &crate::self_tests::boundaries::support::repo_root(),
        &mut out,
    );
    assert!(errors(&out).contains(&"ready_receipt_not_lane_bound"));
}
