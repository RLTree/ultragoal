use super::{lease_issuance::active_record_lanes, scheduler_nodes};
use serde_json::json;
use std::collections::BTreeSet;

fn registry(states: &[(&str, &str)], eligible: &[&str], frontier: &str) -> serde_json::Value {
    json!({
        "pre_adoption_source": {
            "epoch": "ADOPTED-CURRENT",
            "frontier": frontier,
            "eligible_scheduler_nodes": eligible,
        },
        "defaults": {"consumed": {"status": "resolved_current"}},
        "lanes": states.iter().map(|(id, state)| json!({"id": id, "state": state})).collect::<Vec<_>>(),
    })
}

#[test]
fn planned_root_leaves_only_evaluation_ready_for_a_worktree() {
    let nodes = scheduler_nodes(&registry(
        &[("N10", "planned"), ("N11", "ready")],
        &["N11"],
        "N10_ROOT_PLANNED_N11_READY_SOURCE_FRONTIER",
    ))
    .unwrap();

    assert_eq!(nodes.ready, BTreeSet::from(["N11".to_owned()]));
    assert!(nodes.active_worktree_lanes.is_empty());
}

#[test]
fn evaluation_activity_requires_one_active_worktree_lane() {
    for state in ["leased", "candidate", "under_review", "rework", "accepted"] {
        let nodes = scheduler_nodes(&registry(
            &[("N10", "planned"), ("N11", state)],
            &[],
            "N10_ROOT_PLANNED_N11_ACTIVE_SOURCE_FRONTIER",
        ))
        .unwrap();
        assert_eq!(
            nodes.active_worktree_lanes,
            BTreeSet::from(["N11".to_owned()])
        );
    }
}

#[test]
fn integrated_orchestration_preserves_one_active_evaluation_worktree() {
    for state in ["leased", "candidate", "under_review", "rework", "accepted"] {
        let nodes = scheduler_nodes(&registry(
            &[("N10", "integrated"), ("N11", state)],
            &[],
            "N10_INTEGRATED_N11_ACTIVE_SOURCE_FRONTIER",
        ))
        .unwrap();
        assert_eq!(
            nodes.active_worktree_lanes,
            BTreeSet::from(["N11".to_owned()])
        );
    }
}

#[test]
fn lifecycle_rejects_illegal_n11_transitions() {
    let active = scheduler_nodes(&registry(
        &[("N10", "planned"), ("N11", "ready")],
        &["N11"],
        "N10_ROOT_PLANNED_N11_ACTIVE_SOURCE_FRONTIER",
    ));
    let integrating = scheduler_nodes(&registry(
        &[("N10", "planned"), ("N11", "candidate")],
        &[],
        "N10_ROOT_PLANNED_N11_INTEGRATING_ROOT_CLOSURE",
    ));

    assert!(
        active
            .unwrap_err()
            .to_string()
            .contains("illegal active lane lifecycle")
    );
    assert!(
        integrating
            .unwrap_err()
            .to_string()
            .contains("illegal lane lifecycle")
    );
}

#[test]
fn integrated_orchestration_allows_lease_free_evaluation_root_closure() {
    let nodes = scheduler_nodes(&registry(
        &[("N10", "integrated"), ("N11", "integrating")],
        &[],
        "N10_INTEGRATED_N11_INTEGRATING_ROOT_CLOSURE",
    ))
    .unwrap();

    assert!(nodes.ready.is_empty());
    assert!(nodes.active_worktree_lanes.is_empty());
}

#[test]
fn external_evaluation_blocker_allows_only_root_claim_reconciliation() {
    let nodes = scheduler_nodes(&registry(
        &[
            ("N10", "integrated"),
            ("N11", "blocked"),
            ("N12", "planned"),
        ],
        &[],
        "N11_EXTERNAL_BLOCKED_N12_RECONCILIATION_READY",
    ))
    .unwrap();

    assert!(nodes.ready.is_empty());
    assert!(nodes.active_worktree_lanes.is_empty());
}

#[test]
fn external_evaluation_blocker_rejects_unrelated_active_worktree_lanes() {
    let result = scheduler_nodes(&registry(
        &[
            ("N10", "integrated"),
            ("N11", "blocked"),
            ("N12", "planned"),
            ("N13", "leased"),
        ],
        &[],
        "N11_EXTERNAL_BLOCKED_N12_RECONCILIATION_READY",
    ));

    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("unexpected active worktree lanes")
    );
}

#[test]
fn active_leases_are_exactly_one_open_evaluation_worktree() {
    let lanes = BTreeSet::from(["N11".to_owned()]);
    let record = json!({"lane_id":"N11","status":"issued","worktree":"/worktree/n11"});
    assert_eq!(active_record_lanes(&[record], &lanes).unwrap(), lanes);

    let closed = json!({"lane_id":"N11","status":"closed","worktree":"/worktree/n11"});
    let wrong = json!({"lane_id":"N10","status":"issued","worktree":"/worktree/n10"});
    assert!(active_record_lanes(&[], &BTreeSet::from(["N11".to_owned()])).is_err());
    assert!(active_record_lanes(&[closed], &BTreeSet::from(["N11".to_owned()])).is_err());
    assert!(active_record_lanes(&[wrong], &BTreeSet::from(["N11".to_owned()])).is_err());
}
