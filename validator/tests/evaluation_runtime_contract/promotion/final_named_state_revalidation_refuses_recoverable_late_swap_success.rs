#[test]
fn promotion_final_named_state_revalidation_refuses_recoverable_late_swap_success() {
    let baseline_root = root("promotion-final-state-baseline");
    let candidate_root = root("promotion-final-state-candidate");
    let review_root = root("promotion-final-state-swap");
    let saved = review_root.with_extension("consumed-state");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [3_u8; 32];
    let mut review =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    let binding_sha256 = sha('6');
    let attestation = review.issue_attestation(&binding_sha256).unwrap();
    let review_id = review_id(&binding_sha256, &attestation);
    let issued = fs::read(review_root.join("promotion-review.state")).unwrap();

    FilePromotionReviewLedger::set_test_final_validation_pause(review_root.clone(), 10_000);
    let writer = std::thread::spawn(move || {
        review.verify_and_consume(
            &binding_sha256,
            "independent-reviewer",
            &review_id,
            &attestation,
        )
    });
    wait_until("promotion state final validation", || {
        FilePromotionReviewLedger::test_final_validation_is_paused(&review_root)
    });
    fs::rename(review_root.join("promotion-review.state"), &saved).unwrap();
    fs::write(review_root.join("promotion-review.state"), issued).unwrap();
    fs::set_permissions(
        review_root.join("promotion-review.state"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    FilePromotionReviewLedger::release_test_final_validation(&review_root);
    assert!(!writer.join().unwrap());
    fs::remove_file(review_root.join("promotion-review.state")).unwrap();
    fs::rename(&saved, review_root.join("promotion-review.state")).unwrap();

    let consumed = FilePromotionReviewLedger::open(&review_root, key, binding).unwrap();
    assert!(matches!(
        consumed.inspect().unwrap(),
        PromotionLedgerState::Consumed { .. }
    ));
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn execution_readers_wait_for_a_consistent_publication_pair() {
    let root = root("execution-publication-pair");
    let key = [4_u8; 32];
    let binding = execution_binding('b', '1');
    let mut writer =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    FileEvaluationExecutionLedger::set_test_publication_pause(root.clone(), 5_000);
    let writer_thread = std::thread::spawn(move || writer.reserve());
    wait_until("execution state-anchor publication pause", || {
        FileEvaluationExecutionLedger::test_publication_is_paused(&root)
    });

    let (sender, receiver) = mpsc::channel();
    let readers = (0..8)
        .map(|_| {
            let sender = sender.clone();
            let root = root.clone();
            let binding = binding.clone();
            std::thread::spawn(move || {
                let result = FileEvaluationExecutionLedger::open(root, key, binding)
                    .and_then(|ledger| ledger.inspect());
                sender.send(result).unwrap();
            })
        })
        .collect::<Vec<_>>();
    drop(sender);
    assert!(receiver.recv_timeout(Duration::from_millis(50)).is_err());
    FileEvaluationExecutionLedger::release_test_publication(&root);
    writer_thread.join().unwrap().unwrap();
    let states = receiver.into_iter().collect::<Vec<_>>();
    assert_eq!(states.len(), 8);
    assert!(
        states
            .into_iter()
            .all(|state| { matches!(state, Ok(EvaluationLedgerState::Reserved)) })
    );
    for reader in readers {
        reader.join().unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn promotion_readers_wait_for_a_consistent_publication_pair() {
    let baseline_root = root("promotion-pair-baseline");
    let candidate_root = root("promotion-pair-candidate");
    let review_root = root("promotion-pair-review");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let key = [5_u8; 32];
    let mut writer =
        FilePromotionReviewLedger::initialize(&review_root, key, binding.clone()).unwrap();
    let binding_sha256 = sha('6');
    let attestation = writer.issue_attestation(&binding_sha256).unwrap();
    let review_id = review_id(&binding_sha256, &attestation);
    FilePromotionReviewLedger::set_test_publication_pause(review_root.clone(), 5_000);
    let writer_thread = std::thread::spawn(move || {
        writer.verify_and_consume(
            &binding_sha256,
            "independent-reviewer",
            &review_id,
            &attestation,
        )
    });
    wait_until("promotion state-anchor publication pause", || {
        FilePromotionReviewLedger::test_publication_is_paused(&review_root)
    });

    let (sender, receiver) = mpsc::channel();
    let readers = (0..8)
        .map(|_| {
            let sender = sender.clone();
            let review_root = review_root.clone();
            let binding = binding.clone();
            std::thread::spawn(move || {
                let result = FilePromotionReviewLedger::open(review_root, key, binding)
                    .and_then(|ledger| ledger.inspect());
                sender.send(result).unwrap();
            })
        })
        .collect::<Vec<_>>();
    drop(sender);
    assert!(receiver.recv_timeout(Duration::from_millis(50)).is_err());
    FilePromotionReviewLedger::release_test_publication(&review_root);
    assert!(writer_thread.join().unwrap());
    let states = receiver.into_iter().collect::<Vec<_>>();
    assert_eq!(states.len(), 8);
    assert!(
        states
            .into_iter()
            .all(|state| { matches!(state, Ok(PromotionLedgerState::Consumed { .. })) })
    );
    for reader in readers {
        reader.join().unwrap();
    }
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
}

#[test]
fn two_process_execution_worker() {
    let Some(root) = std::env::var_os("HUL_EVAL_EXECUTION_RACE_ROOT") else {
        return;
    };
    if process_barrier().is_err() {
        std::process::exit(82);
    }
    match FileEvaluationExecutionLedger::open(root, [4_u8; 32], execution_binding('b', '1'))
        .and_then(|mut ledger| ledger.reserve())
    {
        Ok(()) => std::process::exit(80),
        Err(error) if error.code() == "evaluation-execution-reservation-conflict" => {
            std::process::exit(81)
        }
        Err(error) => {
            eprintln!("unexpected execution race result: {}", error.code());
            std::process::exit(82)
        }
    }
}
