#[test]
fn promotion_lock_replacement_cannot_create_a_second_mutation_authority() {
    let baseline_root = root("promotion-lock-baseline");
    let candidate_root = root("promotion-lock-candidate");
    let review_root = root("promotion-lock-replacement");
    let saved_lock = review_root.with_extension("saved-promotion-lock");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    fs::rename(review_root.join("promotion-review.lock"), &saved_lock).unwrap();
    let replacement = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(review_root.join("promotion-review.lock"))
        .unwrap();
    drop(replacement);
    fs::set_permissions(
        review_root.join("promotion-review.lock"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(review.require_recovery("lock-replaced").is_err());
    assert!(FilePromotionReviewLedger::open(&review_root, key, binding).is_err());
    fs::remove_file(review_root.join("promotion-review.lock")).unwrap();
    fs::rename(&saved_lock, review_root.join("promotion-review.lock")).unwrap();
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn promotion_final_named_root_revalidation_refuses_orphan_consume_and_read_success() {
    let baseline_root = root("promotion-final-baseline");
    let candidate_root = root("promotion-final-candidate");
    let review_root = root("promotion-final-root-swap");
    let saved = review_root.with_extension("orphaned-authority");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    let binding_sha256 = sha('6');
    let attestation = review.issue_attestation(&binding_sha256).unwrap();
    let review_id = review_id(&binding_sha256, &attestation);

    FilePromotionReviewLedger::set_test_final_validation_pause(review_root.clone(), 10_000);
    let writer = std::thread::spawn(move || {
        review.verify_and_consume(
            &binding_sha256,
            "independent-reviewer",
            &review_id,
            &attestation,
        )
    });
    wait_until("promotion final write validation", || {
        FilePromotionReviewLedger::test_final_validation_is_paused(&review_root)
    });
    fs::rename(&review_root, &saved).unwrap();
    fs::create_dir(&review_root).unwrap();
    fs::set_permissions(&review_root, fs::Permissions::from_mode(0o700)).unwrap();
    FilePromotionReviewLedger::release_test_final_validation(&review_root);
    assert!(!writer.join().unwrap());
    assert!(tree(&review_root).is_empty());
    fs::remove_dir(&review_root).unwrap();
    fs::rename(&saved, &review_root).unwrap();

    let consumed = FilePromotionReviewLedger::open(&review_root, key, binding).unwrap();
    assert!(matches!(
        consumed.inspect().unwrap(),
        PromotionLedgerState::Consumed { .. }
    ));
    FilePromotionReviewLedger::set_test_final_validation_pause(review_root.clone(), 10_000);
    let reader = std::thread::spawn(move || consumed.inspect());
    wait_until("promotion final read validation", || {
        FilePromotionReviewLedger::test_final_validation_is_paused(&review_root)
    });
    fs::rename(&review_root, &saved).unwrap();
    fs::create_dir(&review_root).unwrap();
    fs::set_permissions(&review_root, fs::Permissions::from_mode(0o700)).unwrap();
    FilePromotionReviewLedger::release_test_final_validation(&review_root);
    assert!(reader.join().unwrap().is_err());
    assert!(tree(&review_root).is_empty());
    fs::remove_dir(&review_root).unwrap();
    fs::rename(&saved, &review_root).unwrap();
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn promotion_final_named_lock_and_anchor_revalidation_refuses_late_swap_success() {
    for (label, component) in [
        ("lock", "promotion-review.lock"),
        ("anchor", "promotion-review.anchor.journal"),
    ] {
        let baseline_root = root(&format!("promotion-final-{label}-baseline"));
        let candidate_root = root(&format!("promotion-final-{label}-candidate"));
        let review_root = root(&format!("promotion-final-{label}-swap"));
        let saved = review_root.with_extension(format!("saved-{label}"));
        let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
        let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
        let binding = promotion_binding(&baseline_root, &candidate_root);
        let key = [3_u8; 32];
        let mut review =
            FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
        let binding_sha256 = sha('6');
        let attestation = review.issue_attestation(&binding_sha256).unwrap();
        let review_id = review_id(&binding_sha256, &attestation);

        FilePromotionReviewLedger::set_test_final_validation_pause(review_root.clone(), 10_000);
        let writer = std::thread::spawn(move || {
            review.verify_and_consume(
                &binding_sha256,
                "independent-reviewer",
                &review_id,
                &attestation,
            )
        });
        wait_until("promotion component final validation", || {
            FilePromotionReviewLedger::test_final_validation_is_paused(&review_root)
        });
        fs::rename(review_root.join(component), &saved).unwrap();
        let replacement = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(review_root.join(component))
            .unwrap();
        drop(replacement);
        fs::set_permissions(
            review_root.join(component),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        FilePromotionReviewLedger::release_test_final_validation(&review_root);
        assert!(!writer.join().unwrap());
        assert!(fs::read(review_root.join(component)).unwrap().is_empty());
        fs::remove_file(review_root.join(component)).unwrap();
        fs::rename(&saved, review_root.join(component)).unwrap();
        fs::remove_dir_all(baseline_root).unwrap();
        fs::remove_dir_all(candidate_root).unwrap();
        fs::remove_dir_all(review_root).unwrap();
    }
}
