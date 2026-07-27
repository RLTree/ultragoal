#[test]
fn promotion_anchor_journal_recovers_state_only_rollback_and_rejects_paired_restore() {
    let baseline_root = root("promotion-rollback-baseline");
    let candidate_root = root("promotion-rollback-candidate");
    let state_only_root = root("promotion-state-only-rollback");
    let paired_root = root("promotion-paired-rollback");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];

    let mut review =
        FilePromotionReviewLedger::initialize(&state_only_root, key, binding.clone()).unwrap();
    let old_state = fs::read(state_only_root.join("promotion-review.state")).unwrap();
    review.issue_bound_attestation(&sha('6')).unwrap();
    fs::write(state_only_root.join("promotion-review.state"), old_state).unwrap();
    let recovered =
        FilePromotionReviewLedger::open(&state_only_root, key, binding.clone()).unwrap();
    assert!(matches!(
        recovered.inspect().unwrap(),
        PromotionLedgerState::Issued { .. }
    ));

    let mut review =
        FilePromotionReviewLedger::initialize(&paired_root, key, binding.clone()).unwrap();
    let old_state = fs::read(paired_root.join("promotion-review.state")).unwrap();
    let old_anchor = fs::read(paired_root.join("promotion-review.anchor.journal")).unwrap();
    review.issue_bound_attestation(&sha('6')).unwrap();
    fs::write(paired_root.join("promotion-review.state"), old_state).unwrap();
    fs::write(
        paired_root.join("promotion-review.anchor.journal"),
        old_anchor,
    )
    .unwrap();
    assert!(FilePromotionReviewLedger::open(&paired_root, key, binding).is_err());

    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(state_only_root).unwrap();
    fs::remove_dir_all(paired_root).unwrap();
}

#[test]
fn promotion_journal_tolerates_one_crash_tail_and_repairs_only_on_mutation() {
    let baseline_root = root("promotion-partial-baseline");
    let candidate_root = root("promotion-partial-candidate");
    let review_root = root("promotion-partial-review");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let review = FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    drop(review);
    let anchor_path = review_root.join("promotion-review.anchor.journal");
    let stable_length = fs::metadata(&anchor_path).unwrap().len();
    let mut anchor = fs::OpenOptions::new()
        .append(true)
        .open(&anchor_path)
        .unwrap();
    anchor.write_all(&[0, 0, 0, 1]).unwrap();
    anchor.sync_all().unwrap();
    drop(anchor);

    let before = tree(&review_root);
    let mut reopened = FilePromotionReviewLedger::open(&review_root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Ready
    ));
    assert_eq!(tree(&review_root), before);
    reopened.issue_bound_attestation(&sha('6')).unwrap();
    assert!(fs::metadata(&anchor_path).unwrap().len() > stable_length);
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Issued { .. }
    ));
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn promotion_idempotent_issue_repairs_a_crash_tail_with_a_fresh_authenticated_generation() {
    let baseline_root = root("promotion-idempotent-tail-baseline");
    let candidate_root = root("promotion-idempotent-tail-candidate");
    let review_root = root("promotion-idempotent-tail-review");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let binding_sha256 = sha('6');
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    let attestation = review.issue_bound_attestation(&binding_sha256).unwrap();
    drop(review);

    let anchor_path = review_root.join("promotion-review.anchor.journal");
    let stable_length = fs::metadata(&anchor_path).unwrap().len();
    let mut anchor = fs::OpenOptions::new()
        .append(true)
        .open(&anchor_path)
        .unwrap();
    anchor.write_all(&[0, 0, 0, 1]).unwrap();
    anchor.sync_all().unwrap();
    drop(anchor);

    let before = tree(&review_root);
    let mut reopened = FilePromotionReviewLedger::open(&review_root, key, binding.clone()).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Issued { .. }
    ));
    assert_eq!(tree(&review_root), before);
    assert_eq!(
        reopened.issue_bound_attestation(&binding_sha256).unwrap(),
        attestation
    );
    assert!(fs::metadata(&anchor_path).unwrap().len() > stable_length);
    let current = FilePromotionReviewLedger::open(&review_root, key, binding).unwrap();
    assert!(matches!(
        current.inspect().unwrap(),
        PromotionLedgerState::Issued { .. }
    ));

    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn promotion_stale_protocol_pending_file_is_ignored_read_only_then_removed_on_mutation() {
    let baseline_root = root("promotion-pending-baseline");
    let candidate_root = root("promotion-pending-candidate");
    let review_root = root("promotion-pending-review");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    let binding_sha256 = sha('6');
    let attestation = review.issue_bound_attestation(&binding_sha256).unwrap();
    drop(review);
    let pending = review_root.join(".promotion-review.state.pending.999999.1");
    fs::write(&pending, b"partial-publication").unwrap();
    fs::set_permissions(&pending, fs::Permissions::from_mode(0o600)).unwrap();

    let before = tree(&review_root);
    let mut reopened = FilePromotionReviewLedger::open(&review_root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Issued { .. }
    ));
    assert_eq!(tree(&review_root), before);
    let review_id = review_id(&binding_sha256, &attestation);
    assert!(matches!(
        reopened.consume_attestation(
            &binding_sha256,
            "independent-reviewer",
            &review_id,
            &attestation,
        ),
        Ok(PromotionConsumptionOutcome::Consumed)
    ));
    assert!(!pending.exists());
    assert!(matches!(
        reopened.inspect().unwrap(),
        PromotionLedgerState::Consumed { .. }
    ));
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}
