use super::super::production_input::ProductionSpecPermit;
use super::super::*;
use super::custody::{execution_binding, root};
use super::recovery_setup::{
    PanickingBridge, preparation_spec, production_root, set_panic_effect_marker, sha,
};
use std::fs;

#[test]
fn execution_panic_is_interrupted_and_requires_explicit_recovery() {
    let input_root = production_root("input");
    let ledger_root = production_root("ledger");
    let spec = preparation_spec(&input_root);
    set_panic_effect_marker(Some(input_root.join("fixture-effect")));
    let error = super::super::runtime::ProductionExecutionRequest::new(
        &spec,
        &input_root,
        &ledger_root,
        [6; 32],
        sha('6'),
        sha('8'),
        RuntimeConfiguration::all_unknown(),
    )
    .execute(&mut PanickingBridge)
    .unwrap_err();
    assert_eq!(error.code(), "evaluation-execution-panicked");
    assert_eq!(
        fs::read(input_root.join("fixture-effect")).unwrap(),
        b"fixture-effect-observed-before-panic"
    );

    let binding = EvaluationExecutionBinding::new(EvaluationExecutionBindingRequest {
        live_context_id: spec.live_context_id().to_owned(),
        candidate_id: spec.candidate_id().to_owned(),
        spec_sha256: spec.spec_sha256().to_owned(),
        task_set_sha256: spec.task_set_sha256().to_owned(),
        execution_session_id: sha('6'),
        execution_material_set_sha256: ProductionSpecPermit::test_issue(&spec, &input_root)
            .unwrap()
            .material_set_sha256()
            .to_owned(),
        artifact_root_sha256: sha('8'),
    })
    .unwrap();
    let mut ledger = FileEvaluationExecutionLedger::open(&ledger_root, [6; 32], binding).unwrap();
    assert!(
        matches!(ledger.inspect().unwrap(), EvaluationLedgerState::Interrupted { causal_code } if causal_code == "evaluation-execution-panicked")
    );
    ledger
        .require_recovery("evaluation-panic-recovery-required")
        .unwrap();
    assert_eq!(
        ledger
            .reconcile_authenticated_publication()
            .unwrap_err()
            .code(),
        "evaluation-recovery-publication-unproven"
    );
    assert!(matches!(
        ledger.inspect().unwrap(),
        EvaluationLedgerState::RecoveryRequired { .. }
    ));
    assert!(matches!(
        ledger.reserve_outcome(&sha('7')).unwrap(),
        ExecutionReservationOutcome::RecoveryRequired { .. }
    ));
    set_panic_effect_marker(None);
    fs::remove_dir_all(input_root).unwrap();
    fs::remove_dir_all(ledger_root).unwrap();
}

#[test]
fn execution_interruption_without_authenticated_publication_stays_nonterminal() {
    let root = root("interruption-recovery");
    let key = [7; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    assert_eq!(
        ledger.reserve_outcome(&sha('7')).unwrap(),
        ExecutionReservationOutcome::Acquired
    );
    ledger
        .mark_interrupted("late-effect-revalidation-failed")
        .unwrap();
    drop(ledger);
    let mut reopened = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::Interrupted { .. }
    ));
    reopened
        .require_recovery("late-effect-recovery-required")
        .unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::RecoveryRequired { .. }
    ));
    assert_eq!(
        reopened
            .reconcile_authenticated_publication()
            .unwrap_err()
            .code(),
        "evaluation-recovery-publication-unproven"
    );
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::RecoveryRequired { .. }
    ));
    assert!(matches!(
        reopened.reserve_outcome(&sha('8')).unwrap(),
        ExecutionReservationOutcome::RecoveryRequired { .. }
    ));
    fs::remove_dir_all(root).unwrap();
}
