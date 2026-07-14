#[test]
fn execution_final_named_root_revalidation_refuses_orphan_write_and_read_success() {
    let root = root("execution-final-root-swap");
    let saved = root.with_extension("orphaned-authority");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    ledger.reserve().unwrap();
    ledger.publish_result(sha('8'), sha('9')).unwrap();

    FileEvaluationExecutionLedger::set_test_final_validation_pause(root.clone(), 10_000);
    let writer = std::thread::spawn(move || ledger.complete());
    wait_until("execution final write validation", || {
        FileEvaluationExecutionLedger::test_final_validation_is_paused(&root)
    });
    fs::rename(&root, &saved).unwrap();
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    FileEvaluationExecutionLedger::release_test_final_validation(&root);
    let error = writer.join().unwrap().unwrap_err();
    assert_eq!(error.code(), "evaluation-ledger-descriptor-substituted");
    assert!(tree(&root).is_empty());
    fs::remove_dir(&root).unwrap();
    fs::rename(&saved, &root).unwrap();

    let terminal = FileEvaluationExecutionLedger::open(&root, key, binding.clone()).unwrap();
    assert!(matches!(
        terminal.inspect().unwrap(),
        EvaluationLedgerState::Terminal { .. }
    ));
    FileEvaluationExecutionLedger::set_test_final_validation_pause(root.clone(), 10_000);
    let reader = std::thread::spawn(move || terminal.terminal_proof().map(|_| ()));
    wait_until("execution final read validation", || {
        FileEvaluationExecutionLedger::test_final_validation_is_paused(&root)
    });
    fs::rename(&root, &saved).unwrap();
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    FileEvaluationExecutionLedger::release_test_final_validation(&root);
    let error = reader.join().unwrap().unwrap_err();
    assert_eq!(error.code(), "evaluation-ledger-descriptor-substituted");
    assert!(tree(&root).is_empty());
    fs::remove_dir(&root).unwrap();
    fs::rename(&saved, &root).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_final_named_lock_and_anchor_revalidation_refuses_late_swap_success() {
    for (label, component, expected_code) in [
        (
            "lock",
            "execution.lock",
            "evaluation-ledger-lock-descriptor-substituted",
        ),
        (
            "anchor",
            "execution.anchor.journal",
            "evaluation-anchor-descriptor-substituted",
        ),
    ] {
        let root = root(&format!("execution-final-{label}-swap"));
        let saved = root.with_extension(format!("saved-{label}"));
        let key = [8_u8; 32];
        let binding = execution_binding('b', '1');
        let mut ledger =
            FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
        ledger.reserve().unwrap();
        ledger.publish_result(sha('8'), sha('9')).unwrap();

        FileEvaluationExecutionLedger::set_test_final_validation_pause(root.clone(), 10_000);
        let writer = std::thread::spawn(move || ledger.complete());
        wait_until("execution component final validation", || {
            FileEvaluationExecutionLedger::test_final_validation_is_paused(&root)
        });
        fs::rename(root.join(component), &saved).unwrap();
        let replacement = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(component))
            .unwrap();
        drop(replacement);
        fs::set_permissions(root.join(component), fs::Permissions::from_mode(0o600)).unwrap();
        FileEvaluationExecutionLedger::release_test_final_validation(&root);
        let error = writer.join().unwrap().unwrap_err();
        assert_eq!(error.code(), expected_code);
        assert!(fs::read(root.join(component)).unwrap().is_empty());
        fs::remove_file(root.join(component)).unwrap();
        fs::rename(&saved, root.join(component)).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn execution_final_named_state_revalidation_refuses_recoverable_late_swap_success() {
    let root = root("execution-final-state-swap");
    let saved = root.with_extension("terminal-state");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    ledger.reserve().unwrap();
    ledger.publish_result(sha('8'), sha('9')).unwrap();
    let published = fs::read(root.join("execution.state")).unwrap();

    FileEvaluationExecutionLedger::set_test_final_validation_pause(root.clone(), 10_000);
    let writer = std::thread::spawn(move || ledger.complete());
    wait_until("execution state final validation", || {
        FileEvaluationExecutionLedger::test_final_validation_is_paused(&root)
    });
    fs::rename(root.join("execution.state"), &saved).unwrap();
    fs::write(root.join("execution.state"), published).unwrap();
    fs::set_permissions(
        root.join("execution.state"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    FileEvaluationExecutionLedger::release_test_final_validation(&root);
    let error = writer.join().unwrap().unwrap_err();
    assert_eq!(error.code(), "evaluation-ledger-final-current-changed");
    fs::remove_file(root.join("execution.state")).unwrap();
    fs::rename(&saved, root.join("execution.state")).unwrap();

    let terminal = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        terminal.inspect().unwrap(),
        EvaluationLedgerState::Terminal { .. }
    ));
    fs::remove_dir_all(root).unwrap();
}

fn terminal_ledger(
    root: &Path,
    key: [u8; 32],
    candidate: char,
    session: char,
    run: char,
) -> FileEvaluationExecutionLedger {
    let binding = execution_binding(candidate, session);
    let mut ledger = FileEvaluationExecutionLedger::initialize(root, key, binding).unwrap();
    ledger.reserve().unwrap();
    ledger.publish_result(sha(run), sha('9')).unwrap();
    ledger.complete().unwrap();
    ledger
}

fn promotion_binding(baseline_root: &Path, candidate_root: &Path) -> PromotionLedgerBinding {
    let baseline =
        FileEvaluationExecutionLedger::open(baseline_root, [1_u8; 32], execution_binding('b', '1'))
            .unwrap();
    let candidate = FileEvaluationExecutionLedger::open(
        candidate_root,
        [2_u8; 32],
        execution_binding('c', '2'),
    )
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
