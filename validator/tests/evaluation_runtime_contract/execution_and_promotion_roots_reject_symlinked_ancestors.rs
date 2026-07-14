#[test]
fn execution_and_promotion_roots_reject_symlinked_ancestors() {
    let parent = root("ledger-symlink-ancestor");
    let alias = parent.with_extension("ancestor-alias");
    let execution_root = parent.join("execution");
    let review_root = parent.join("review");
    fs::create_dir(&execution_root).unwrap();
    fs::create_dir(&review_root).unwrap();
    fs::set_permissions(&execution_root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(&review_root, fs::Permissions::from_mode(0o700)).unwrap();
    symlink(&parent, &alias).unwrap();

    let execution_error = FileEvaluationExecutionLedger::initialize(
        alias.join("execution"),
        [7_u8; 32],
        execution_binding('b', '1'),
    )
    .err()
    .unwrap();
    assert_eq!(execution_error.code(), "evaluation-ledger-root-open-failed");

    let baseline_root = root("symlink-ancestor-baseline");
    let candidate_root = root("symlink-ancestor-candidate");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let promotion_error =
        FilePromotionReviewLedger::initialize(alias.join("review"), [3_u8; 32], binding)
            .err()
            .unwrap();
    assert_eq!(promotion_error.code(), "promotion-ledger-root-unsafe");

    fs::remove_file(alias).unwrap();
    fs::remove_dir_all(parent).unwrap();
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
}

#[test]
fn execution_descriptor_scan_cannot_be_redirected_away_from_an_unknown_entry() {
    let root = root("execution-descriptor-directory-scan");
    let saved = root.with_extension("descriptor-authority");
    let key = [7_u8; 32];
    let binding = execution_binding('b', '1');
    let ledger = FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    fs::write(root.join("unrecognized-authority"), b"must be observed").unwrap();

    FileEvaluationExecutionLedger::set_test_directory_scan_pause(root.clone(), 10_000);
    FileEvaluationExecutionLedger::set_test_final_validation_pause(root.clone(), 10_000);
    let reader = std::thread::spawn(move || ledger.inspect());
    wait_until("execution descriptor directory scan", || {
        FileEvaluationExecutionLedger::test_directory_scan_is_paused(&root)
    });
    fs::rename(&root, &saved).unwrap();
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    FileEvaluationExecutionLedger::release_test_directory_scan(&root);
    wait_until("execution descriptor scan rejection", || {
        reader.is_finished()
            || FileEvaluationExecutionLedger::test_final_validation_is_paused(&root)
    });
    let reached_final_validation =
        FileEvaluationExecutionLedger::test_final_validation_is_paused(&root);
    assert!(tree(&root).is_empty());
    fs::remove_dir(&root).unwrap();
    fs::rename(&saved, &root).unwrap();
    FileEvaluationExecutionLedger::release_test_final_validation(&root);
    let error = reader.join().unwrap().unwrap_err();
    assert!(
        !reached_final_validation,
        "a path-based scan reached final validation before observing the descriptor-root entry"
    );
    assert_eq!(error.code(), "evaluation-ledger-unknown-or-pending-entry");
    fs::remove_file(root.join("unrecognized-authority")).unwrap();
    let reopened = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::Initialized
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn promotion_descriptor_scan_cannot_be_redirected_away_from_an_unknown_entry() {
    let baseline_root = root("promotion-scan-baseline");
    let candidate_root = root("promotion-scan-candidate");
    let review_root = root("promotion-descriptor-directory-scan");
    let saved = review_root.with_extension("descriptor-authority");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let review =
        FilePromotionReviewLedger::initialize(&review_root, [3_u8; 32], binding.clone()).unwrap();
    fs::write(
        review_root.join("unrecognized-authority"),
        b"must be observed",
    )
    .unwrap();

    FilePromotionReviewLedger::set_test_directory_scan_pause(review_root.clone(), 10_000);
    FilePromotionReviewLedger::set_test_final_validation_pause(review_root.clone(), 10_000);
    let reader = std::thread::spawn(move || review.inspect());
    wait_until("promotion descriptor directory scan", || {
        FilePromotionReviewLedger::test_directory_scan_is_paused(&review_root)
    });
    fs::rename(&review_root, &saved).unwrap();
    fs::create_dir(&review_root).unwrap();
    fs::set_permissions(&review_root, fs::Permissions::from_mode(0o700)).unwrap();
    FilePromotionReviewLedger::release_test_directory_scan(&review_root);
    wait_until("promotion descriptor scan rejection", || {
        reader.is_finished()
            || FilePromotionReviewLedger::test_final_validation_is_paused(&review_root)
    });
    let reached_final_validation =
        FilePromotionReviewLedger::test_final_validation_is_paused(&review_root);
    assert!(tree(&review_root).is_empty());
    fs::remove_dir(&review_root).unwrap();
    fs::rename(&saved, &review_root).unwrap();
    FilePromotionReviewLedger::release_test_final_validation(&review_root);
    let error = reader.join().unwrap().unwrap_err();
    assert!(
        !reached_final_validation,
        "a path-based scan reached final validation before observing the descriptor-root entry"
    );
    assert_eq!(error.code(), "promotion-ledger-unknown-or-pending-entry");
    fs::remove_file(review_root.join("unrecognized-authority")).unwrap();
    let reopened = FilePromotionReviewLedger::open(&review_root, [3_u8; 32], binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Ready
    ));
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn distinct_review_ledger_is_one_shot_and_rejects_replay() {
    let baseline_root = root("promotion-baseline");
    let candidate_root = root("promotion-candidate");
    let review_root = root("promotion-review");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, [3_u8; 32], binding).unwrap();
    let binding_sha256 = sha('6');
    let attestation = review.issue_attestation(&binding_sha256).unwrap();
    let review_id = review_id(&binding_sha256, &attestation);
    assert!(review.verify_and_consume(
        &binding_sha256,
        "independent-reviewer",
        &review_id,
        &attestation,
    ));
    assert!(!review.verify_and_consume(
        &binding_sha256,
        "independent-reviewer",
        &review_id,
        &attestation,
    ));
    assert!(matches!(
        review.inspect().unwrap(),
        PromotionLedgerState::Consumed { .. }
    ));
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}
