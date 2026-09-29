use super::super::*;
use super::custody::{execution_binding, root};
use sha2::Digest;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::time::{Duration, Instant};

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

fn wait_for_pause(root: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !FilePromotionReviewLedger::test_final_validation_is_paused(root) {
        assert!(
            Instant::now() < deadline,
            "promotion final validation did not pause"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn promotion_symlinked_root_is_refused_before_review_authority_issues() {
    let parent = root("promotion-symlink-parent");
    let actual = parent.join("review");
    fs::create_dir(&actual).unwrap();
    fs::set_permissions(&actual, fs::Permissions::from_mode(0o700)).unwrap();
    let alias = parent.with_extension("alias");
    symlink(&parent, &alias).unwrap();
    let baseline = root("promotion-symlink-baseline");
    let candidate = root("promotion-symlink-candidate");
    terminal(&baseline, [1; 32], 'b', '1', '4');
    terminal(&candidate, [2; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline, &candidate);
    assert!(FilePromotionReviewLedger::initialize(alias.join("review"), [3; 32], binding).is_err());
    fs::remove_file(alias).unwrap();
    for path in [parent, baseline, candidate] {
        fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn promotion_lock_replacement_refuses_a_second_mutation_authority() {
    let baseline = root("promotion-lock-baseline");
    let candidate = root("promotion-lock-candidate");
    let review = root("promotion-lock-replacement");
    terminal(&baseline, [1; 32], 'b', '1', '4');
    terminal(&candidate, [2; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline, &candidate);
    let mut ledger =
        FilePromotionReviewLedger::initialize(&review, [3; 32], binding.clone()).unwrap();
    let saved = review.with_extension("saved-promotion-lock");
    fs::rename(review.join("promotion-review.lock"), &saved).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(review.join("promotion-review.lock"))
        .unwrap();
    fs::set_permissions(
        review.join("promotion-review.lock"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(ledger.require_recovery("lock-replaced").is_err());
    assert!(FilePromotionReviewLedger::open(&review, [3; 32], binding).is_err());
    fs::remove_file(review.join("promotion-review.lock")).unwrap();
    fs::rename(saved, review.join("promotion-review.lock")).unwrap();
    for path in [baseline, candidate, review] {
        fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn promotion_final_named_state_swap_refuses_late_consume_success() {
    let baseline = root("promotion-final-state-baseline");
    let candidate = root("promotion-final-state-candidate");
    let review = root("promotion-final-state-swap");
    terminal(&baseline, [1; 32], 'b', '1', '4');
    terminal(&candidate, [2; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline, &candidate);
    let mut ledger =
        FilePromotionReviewLedger::initialize(&review, [3; 32], binding.clone()).unwrap();
    let binding_sha256 = sha('6');
    let attestation = ledger.issue_bound_attestation(&binding_sha256).unwrap();
    let review_id = format!(
        "sha256:{:x}",
        sha2::Sha256::digest(format!("promotion-review|{binding_sha256}|{attestation}").as_bytes())
    );
    let issued = fs::read(review.join("promotion-review.state")).unwrap();
    FilePromotionReviewLedger::set_test_final_validation_pause(review.clone(), 5_000);
    let writer = std::thread::spawn(move || {
        ledger.consume_attestation(
            &binding_sha256,
            "independent-reviewer",
            &review_id,
            &attestation,
        )
    });
    wait_for_pause(&review);
    let saved = review.with_extension("saved-promotion-state");
    fs::rename(review.join("promotion-review.state"), &saved).unwrap();
    fs::write(review.join("promotion-review.state"), issued).unwrap();
    fs::set_permissions(
        review.join("promotion-review.state"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    FilePromotionReviewLedger::release_test_final_validation(&review);
    assert!(writer.join().unwrap().is_err());
    fs::remove_file(review.join("promotion-review.state")).unwrap();
    fs::rename(saved, review.join("promotion-review.state")).unwrap();
    let consumed = FilePromotionReviewLedger::open(&review, [3; 32], binding).unwrap();
    assert!(matches!(
        consumed.inspect().unwrap(),
        PromotionLedgerState::Consumed { .. }
    ));
    for path in [baseline, candidate, review] {
        fs::remove_dir_all(path).unwrap();
    }
}
