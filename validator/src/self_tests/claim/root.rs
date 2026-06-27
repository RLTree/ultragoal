use crate::audit::contract::Failure;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn semantic_loader_and_automation_tick_reject_bad_boundaries() {
    let root = crate::self_tests::boundaries::support::temp_root("semantic-loader");
    std::fs::create_dir_all(root.join("receipts")).expect("receipts");
    let receipt =
        json!({"path":"../escape.json","digest":crate::self_tests::boundaries::support::sha('f')});
    let err = crate::claim_semantics::semantic::receipt::loader::load(&root, &receipt)
        .err()
        .expect("path escape rejected");
    assert!(!err.is_empty());
    let no_digest_path = root.join("receipts/no-digest.json");
    std::fs::write(&no_digest_path, br#"{"claim_id":"NO-DIGEST"}"#).expect("no digest receipt");
    let loaded = crate::claim_semantics::semantic::receipt::loader::load(
        &root,
        &json!({"path":"receipts/no-digest.json"}),
    )
    .expect("path receipt without digest");
    assert!(!loaded.inline);
    assert_eq!(loaded.value["claim_id"], "NO-DIGEST");

    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::automation_tick::check(
        &json!({
            "status": "active",
            "thread_id": "",
            "goal_id": "",
            "repo_root": "",
            "required_tools": [],
            "required_skills": [],
            "evidence_cursors": []
        }),
        &mut out,
    );
    assert_eq!(out.len(), 1);
    crate::claim_semantics::automation_tick::check(
        &json!({
            "freshness_policy": {
                "clock_at_validation": "2026-06-25T00:00:00Z",
                "max_tick_age_minutes": 5,
                "max_success_age_minutes": 5
            },
            "last_tick_at": "2026-06-25T00:01:00Z",
            "last_success_at": "2026-06-25T00:00:00Z",
            "validator_computed_drift_verdict": "fresh",
            "drift_verdict": "fresh"
        }),
        &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "automation_tick_future_timestamp")
    );
    crate::claim_semantics::automation_tick::check(
        &json!({
            "freshness_policy": {
                "clock_at_validation": "2026-06-25T00:10:00Z",
                "max_tick_age_minutes": 1,
                "max_success_age_minutes": 1
            },
            "last_tick_at": "2026-06-25T00:00:00Z",
            "last_success_at": "2026-06-25T00:00:00Z",
            "validator_computed_drift_verdict": "stale",
            "drift_verdict": "stale"
        }),
        &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.error == "automation_tick_stale_or_missing")
    );
    out.clear();
    crate::claim_semantics::automation_tick::check(
        &json!({
            "freshness_policy": {"clock_at_validation": "2026-06-25T00:10:00Z"},
            "last_tick_at": "bad",
            "last_success_at": "2026-06-25T00:00:00Z"
        }),
        &mut out,
    );
    assert!(out.iter().any(|failure| failure.detail == "/last_tick_at"));
    out.clear();
    crate::claim_semantics::automation_tick::check(
        &json!({
            "freshness_policy": {"clock_at_validation": "2026-06-25T00:10:00Z"},
            "last_tick_at": "2026-06-25T00:00:00Z",
            "last_success_at": "bad"
        }),
        &mut out,
    );
    assert!(
        out.iter()
            .any(|failure| failure.detail == "/last_success_at")
    );
    out.clear();
    let fresh_tick = json!({
        "freshness_policy": {
            "clock_at_validation": "2026-06-25T00:10:00Z",
            "max_tick_age_minutes": 15,
            "max_success_age_minutes": 15
        },
        "last_tick_at": "2026-06-25T00:09:00Z",
        "last_success_at": "2026-06-25T00:08:00Z",
        "validator_computed_drift_verdict": "fresh",
        "drift_verdict": "fresh"
    });
    crate::claim_semantics::automation_tick::check(&fresh_tick, &mut out);
    assert!(out.is_empty(), "{out:?}");
    let mut mismatch = fresh_tick;
    mismatch["drift_verdict"] = json!("stale");
    crate::claim_semantics::automation_tick::check(&mismatch, &mut out);
    assert!(
        out.iter()
            .any(|failure| failure.error == "automation_timestamps_stale_but_claim_fresh")
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn plugin_policy_handles_manifest_and_flow_boundaries() {
    let root = PathBuf::from(".");
    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::plugin_policy::check_plugin(
        &json!({
            "plugin_manifest": {
                "name": "harness-ultragoal",
                "version": "0.0.11",
                "skills": [],
                "agents": []
            },
            "plugin_flow": {
                "required_edges": [],
                "completion_receipt": {}
            }
        }),
        &root,
        &mut out,
    );
    assert!(!out.is_empty());
}
