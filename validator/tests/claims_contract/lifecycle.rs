use super::claims::{DecisionLedger, DecisionStatus};
use super::scenario::{
    candidate_id, context_id, definitions, now, observations_for, pass_claim, reviewer, submit,
};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn repair_invalidates_only_present_dependent_decisions_and_rerun_recloses_them() {
    let definitions = definitions();
    let mut ledger = super::scenario::semantic_model_ledger();
    pass_claim(&mut ledger, &definitions, "CL-SOURCE", "initial");
    pass_claim(&mut ledger, &definitions, "CL-PACKAGE", "initial");
    pass_claim(&mut ledger, &definitions, "CL-INSTALL", "initial");
    pass_claim(&mut ledger, &definitions, "CL-ORCHESTRATION", "initial");
    let old_package_evidence = ledger
        .projection(&definitions)
        .decisions
        .iter()
        .find(|decision| decision.claim_id == "CL-PACKAGE")
        .expect("package")
        .evidence_ids
        .clone();

    let invalidated = ledger
        .invalidate_for_repair(&definitions, "CL-PACKAGE")
        .expect("invalidate");
    assert_eq!(
        invalidated,
        vec!["CL-INSTALL".to_owned(), "CL-PACKAGE".to_owned()]
    );
    assert_eq!(
        ledger
            .projection(&definitions)
            .decisions
            .iter()
            .find(|decision| decision.claim_id == "CL-ORCHESTRATION")
            .expect("orchestration")
            .status,
        DecisionStatus::Passed
    );
    let stale_rerun = ledger.decide(
        &definitions,
        "CL-PACKAGE",
        context_id(),
        candidate_id(),
        now(),
        &reviewer(&definitions, "CL-PACKAGE"),
        &old_package_evidence,
    );
    assert_eq!(stale_rerun.status, DecisionStatus::Rejected);
    assert!(
        stale_rerun
            .reasons
            .iter()
            .any(|reason| reason.contains("invalidated-by-repair"))
    );

    pass_claim(&mut ledger, &definitions, "CL-PACKAGE", "rerun");
    pass_claim(&mut ledger, &definitions, "CL-INSTALL", "rerun");
    assert_eq!(
        ledger
            .projection(&definitions)
            .decisions
            .iter()
            .find(|decision| decision.claim_id == "CL-INSTALL")
            .expect("install")
            .status,
        DecisionStatus::Passed
    );
}

#[test]
fn rejected_obligations_remain_visible_and_projection_is_deterministic_zero_write() {
    let definitions = definitions();
    let mut ledger = super::scenario::semantic_model_ledger();
    pass_claim(&mut ledger, &definitions, "CL-SOURCE", "projection-source");
    let mut observations = observations_for(&definitions, "CL-PACKAGE", "projection");
    observations.remove(0);
    let ids = submit(&mut ledger, observations);
    let decision = ledger.decide(
        &definitions,
        "CL-PACKAGE",
        context_id(),
        candidate_id(),
        now(),
        &reviewer(&definitions, "CL-PACKAGE"),
        &ids,
    );
    assert_eq!(decision.status, DecisionStatus::Rejected);
    let projection = ledger.projection(&definitions);
    assert_eq!(projection.rejected_observations.len(), ids.len());
    assert!(
        projection
            .rejected_observations
            .iter()
            .all(|item| !item.reasons.is_empty())
    );

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ultragoal-claims-projection-{stamp}"));
    fs::create_dir(&dir).expect("create temp");
    fs::write(dir.join("sentinel"), b"unchanged").expect("sentinel");
    let before = fs::read_dir(&dir).expect("read before").count();
    let first = serde_json::to_vec(&ledger.projection(&definitions)).expect("projection");
    let second = serde_json::to_vec(&ledger.projection(&definitions)).expect("projection again");
    assert_eq!(first, second);
    assert_eq!(before, fs::read_dir(&dir).expect("read after").count());
    fs::remove_dir_all(dir).expect("remove temp");
}
