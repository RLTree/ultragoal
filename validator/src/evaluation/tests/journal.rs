use super::super::*;
use super::custody::{execution_binding, root};
use std::fs;

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn terminal(root: &std::path::Path, key: [u8; 32], candidate: char, session: char, run: char) {
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(root, key, execution_binding(candidate, session))
            .unwrap();
    ledger.reserve().unwrap();
    ledger.publish_result(sha(run), sha('9')).unwrap();
    ledger.complete().unwrap();
}

fn binding(baseline: &std::path::Path, candidate: &std::path::Path) -> PromotionLedgerBinding {
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

#[test]
fn promotion_anchor_recovers_state_only_rollback_and_rejects_paired_restore() {
    let baseline = root("journal-baseline");
    let candidate = root("journal-candidate");
    terminal(&baseline, [1; 32], 'b', '1', '4');
    terminal(&candidate, [2; 32], 'c', '2', '5');
    let binding = binding(&baseline, &candidate);
    let key = [3; 32];

    let state_only = root("journal-state-only");
    let mut state_ledger =
        FilePromotionReviewLedger::initialize(&state_only, key, binding.clone()).unwrap();
    let original_state = fs::read(state_only.join("promotion-review.state")).unwrap();
    state_ledger.issue_bound_attestation(&sha('6')).unwrap();
    fs::write(state_only.join("promotion-review.state"), original_state).unwrap();
    let recovered = FilePromotionReviewLedger::open(&state_only, key, binding.clone()).unwrap();
    assert!(matches!(
        recovered.inspect().unwrap(),
        PromotionLedgerState::Issued { .. }
    ));

    let paired = root("journal-paired");
    let mut paired_ledger =
        FilePromotionReviewLedger::initialize(&paired, key, binding.clone()).unwrap();
    let original_state = fs::read(paired.join("promotion-review.state")).unwrap();
    let original_anchor = fs::read(paired.join("promotion-review.anchor.journal")).unwrap();
    paired_ledger.issue_bound_attestation(&sha('6')).unwrap();
    fs::write(paired.join("promotion-review.state"), original_state).unwrap();
    fs::write(
        paired.join("promotion-review.anchor.journal"),
        original_anchor,
    )
    .unwrap();
    assert!(FilePromotionReviewLedger::open(&paired, key, binding).is_err());

    drop((state_ledger, recovered, paired_ledger));
    for path in [baseline, candidate, state_only, paired] {
        fs::remove_dir_all(path).unwrap();
    }
}
