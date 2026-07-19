use super::super::*;
use super::custody::{execution_binding, root};
use std::fs;

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn promotion_binding(
    baseline: &std::path::Path,
    candidate: &std::path::Path,
) -> PromotionLedgerBinding {
    let baseline =
        FileEvaluationExecutionLedger::open(baseline, [1; 32], execution_binding('b', '1'))
            .unwrap();
    let candidate =
        FileEvaluationExecutionLedger::open(candidate, [2; 32], execution_binding('c', '2'))
            .unwrap();
    PromotionLedgerBinding::from_terminal_proofs(
        "promotion-authority",
        "independent-reviewer",
        sha('3'),
        baseline.terminal_proof().unwrap(),
        candidate.terminal_proof().unwrap(),
    )
    .unwrap()
}

fn terminal(root: &std::path::Path, key: [u8; 32], candidate: char, session: char, run: char) {
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(root, key, execution_binding(candidate, session))
            .unwrap();
    ledger.reserve().unwrap();
    ledger.publish_result(sha(run), sha('9')).unwrap();
    ledger.complete().unwrap();
}

#[test]
fn execution_initialization_recovers_each_authenticated_durable_prefix() {
    for stage in [
        "lock",
        "anchor_temporary",
        "anchor_named",
        "state_temporary",
        "state_named",
    ] {
        let root = root(&format!("execution-init-{stage}"));
        let binding = execution_binding('b', '1');
        FileEvaluationExecutionLedger::set_test_initialization_interruption(root.clone(), stage);
        let error =
            FileEvaluationExecutionLedger::initialize(&root, [7; 32], binding.clone()).unwrap_err();
        assert_eq!(error.code(), "evaluation-ledger-initialization-interrupted");
        let first =
            FileEvaluationExecutionLedger::initialize(&root, [7; 32], binding.clone()).unwrap();
        assert!(matches!(
            first.inspect().unwrap(),
            EvaluationLedgerState::Initialized
        ));
        let settled = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(settled.len(), 3);
        let second =
            FileEvaluationExecutionLedger::initialize(&root, [7; 32], binding.clone()).unwrap();
        assert!(matches!(
            second.inspect().unwrap(),
            EvaluationLedgerState::Initialized
        ));
        assert!(FileEvaluationExecutionLedger::initialize(&root, [8; 32], binding).is_err());
        drop((first, second));
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn promotion_initialization_recovers_each_authenticated_durable_prefix() {
    let baseline = root("promotion-init-baseline");
    let candidate = root("promotion-init-candidate");
    terminal(&baseline, [1; 32], 'b', '1', '4');
    terminal(&candidate, [2; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline, &candidate);
    for stage in [
        "lock",
        "anchor_temporary",
        "anchor_named",
        "state_temporary",
        "state_named",
    ] {
        let review = root(&format!("promotion-init-{stage}"));
        FilePromotionReviewLedger::set_test_initialization_interruption(review.clone(), stage);
        let error =
            FilePromotionReviewLedger::initialize(&review, [6; 32], binding.clone()).unwrap_err();
        assert_eq!(error.code(), "promotion-ledger-initialization-interrupted");
        let first =
            FilePromotionReviewLedger::initialize(&review, [6; 32], binding.clone()).unwrap();
        assert!(matches!(
            first.inspect().unwrap(),
            PromotionLedgerState::Ready
        ));
        let second =
            FilePromotionReviewLedger::initialize(&review, [6; 32], binding.clone()).unwrap();
        assert!(matches!(
            second.inspect().unwrap(),
            PromotionLedgerState::Ready
        ));
        assert!(FilePromotionReviewLedger::initialize(&review, [7; 32], binding.clone()).is_err());
        drop((first, second));
        fs::remove_dir_all(review).unwrap();
    }
    fs::remove_dir_all(baseline).unwrap();
    fs::remove_dir_all(candidate).unwrap();
}
