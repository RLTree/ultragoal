use super::super::*;
use std::collections::BTreeSet;

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn task(
    controls: BTreeSet<PerturbationControl>,
    known_training: BTreeSet<String>,
) -> EvaluationTask {
    EvaluationTask::new_with_provenance(
        EvaluationTaskDefinition {
            task_id: "core".to_owned(),
            requirement_id: "REQ-core".to_owned(),
            behavior_id: "behavior-core".to_owned(),
            fixture_id: "fixture-core".to_owned(),
            dataset: BoundInput::regular("datasets/core.json", sha('a'), 32),
            scorer_id: "scorer-core".to_owned(),
            scorer_digest_sha256: sha('b'),
            perturbation_controls: controls,
            representative: true,
        },
        EvaluationDatasetProvenance {
            dataset_provenance_sha256: sha('c'),
            known_training_corpus_sha256s: known_training,
        },
    )
}

fn audit(task: EvaluationTask) -> TaskAudit {
    let spec = EvaluationSpec::new(sha('d'), sha('e'), "domain-controls", vec![task]).unwrap();
    spec.audit(&sha('d'), &sha('e'))
}

#[test]
fn contaminated_dataset_cannot_become_an_eligible_evaluation() {
    let audit = audit(task(
        PerturbationControl::REQUIRED.into_iter().collect(),
        BTreeSet::from([sha('a')]),
    ));
    assert!(!audit.eligible());
    assert!(
        audit
            .findings()
            .contains(&"evaluation-dataset-contamination-detected".to_owned())
    );
}

#[test]
fn missing_reward_hacking_control_blocks_the_run_boundary() {
    let mut controls: BTreeSet<_> = PerturbationControl::REQUIRED.into_iter().collect();
    controls.remove(&PerturbationControl::ReceiptProduction);
    let audit = audit(task(controls, BTreeSet::new()));
    assert!(!audit.eligible());
    assert!(
        audit
            .findings()
            .contains(&"evaluation-perturbation-control-missing:receipt-production".to_owned())
    );
}
