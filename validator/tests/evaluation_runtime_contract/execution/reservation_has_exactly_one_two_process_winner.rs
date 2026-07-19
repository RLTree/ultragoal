#[test]
fn execution_reservation_has_exactly_one_two_process_winner() {
    let ledger_root = root("execution-race");
    let ledger = FileEvaluationExecutionLedger::initialize(
        &ledger_root,
        [4_u8; 32],
        execution_binding('b', '1'),
    )
    .unwrap();
    drop(ledger);
    let exe = std::env::current_exe().unwrap();
    let barrier = root("execution-race-barrier");
    let mut children = (0..2)
        .map(|index| {
            Command::new(&exe)
                .args(["--exact", "two_process_execution_worker", "--nocapture"])
                .env("HUL_EVAL_EXECUTION_RACE_ROOT", &ledger_root)
                .env("HUL_EVAL_RACE_BARRIER", &barrier)
                .env("HUL_EVAL_RACE_PARTICIPANT", index.to_string())
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    release_process_barrier(&barrier, 2);
    let mut statuses = children
        .iter_mut()
        .map(|child| child.wait().unwrap().code().unwrap())
        .collect::<Vec<_>>();
    statuses.sort_unstable();
    assert_eq!(statuses, vec![80, 81]);
    fs::remove_dir_all(ledger_root).unwrap();
    fs::remove_dir_all(barrier).unwrap();
}

#[test]
fn two_process_review_worker() {
    let Some(review_root) = std::env::var_os("HUL_EVAL_REVIEW_RACE_ROOT") else {
        return;
    };
    if process_barrier().is_err() {
        std::process::exit(85);
    }
    let baseline_root = PathBuf::from(std::env::var_os("HUL_EVAL_BASELINE_ROOT").unwrap());
    let candidate_root = PathBuf::from(std::env::var_os("HUL_EVAL_CANDIDATE_ROOT").unwrap());
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let mut ledger = match FilePromotionReviewLedger::open(review_root, [5_u8; 32], binding) {
        Ok(ledger) => ledger,
        Err(error) => {
            eprintln!("unexpected review race open result: {}", error.code());
            std::process::exit(85)
        }
    };
    let binding_sha256 = sha('6');
    let attestation = std::env::var("HUL_EVAL_ATTESTATION").unwrap();
    let review_id = review_id(&binding_sha256, &attestation);
    if matches!(
        ledger.consume_attestation(
            &binding_sha256,
            "independent-reviewer",
            &review_id,
            &attestation,
        ),
        Ok(PromotionConsumptionOutcome::Consumed)
    ) {
        std::process::exit(83);
    }
    match ledger.inspect() {
        Ok(PromotionLedgerState::Consumed { .. }) => std::process::exit(84),
        Ok(state) => {
            eprintln!("unexpected review race loser state: {state:?}");
            std::process::exit(85)
        }
        Err(error) => {
            eprintln!("unexpected review race inspect result: {}", error.code());
            std::process::exit(85)
        }
    }
}

#[test]
fn review_consumption_has_exactly_one_two_process_winner() {
    let baseline_root = root("review-race-baseline");
    let candidate_root = root("review-race-candidate");
    let review_root = root("review-race");
    let _baseline = terminal_ledger(&baseline_root, [1_u8; 32], 'b', '1', '4');
    let _candidate = terminal_ledger(&candidate_root, [2_u8; 32], 'c', '2', '5');
    let binding = promotion_binding(&baseline_root, &candidate_root);
    let mut ledger =
        FilePromotionReviewLedger::initialize(&review_root, [5_u8; 32], binding).unwrap();
    let attestation = ledger.issue_bound_attestation(&sha('6')).unwrap();
    drop(ledger);
    let exe = std::env::current_exe().unwrap();
    let barrier = root("review-race-barrier");
    let mut children = (0..2)
        .map(|index| {
            Command::new(&exe)
                .args(["--exact", "two_process_review_worker", "--nocapture"])
                .env("HUL_EVAL_REVIEW_RACE_ROOT", &review_root)
                .env("HUL_EVAL_BASELINE_ROOT", &baseline_root)
                .env("HUL_EVAL_CANDIDATE_ROOT", &candidate_root)
                .env("HUL_EVAL_ATTESTATION", &attestation)
                .env("HUL_EVAL_RACE_BARRIER", &barrier)
                .env("HUL_EVAL_RACE_PARTICIPANT", index.to_string())
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    release_process_barrier(&barrier, 2);
    let mut statuses = children
        .iter_mut()
        .map(|child| child.wait().unwrap().code().unwrap())
        .collect::<Vec<_>>();
    statuses.sort_unstable();
    assert_eq!(statuses, vec![83, 84]);
    fs::remove_dir_all(baseline_root).unwrap();
    fs::remove_dir_all(candidate_root).unwrap();
    fs::remove_dir_all(review_root).unwrap();
    fs::remove_dir_all(barrier).unwrap();
}
