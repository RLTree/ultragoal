fn execution_binding(candidate: char, session: char) -> EvaluationExecutionBinding {
    EvaluationExecutionBinding::new(EvaluationExecutionBindingRequest {
        live_context_id: sha('a'),
        candidate_id: sha(candidate),
        spec_sha256: sha('c'),
        task_set_sha256: sha('d'),
        execution_session_id: sha(session),
        execution_material_set_sha256: sha('e'),
        artifact_root_sha256: sha('f'),
    })
    .unwrap()
}

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                fs::read(entry.path()).unwrap_or_default(),
            )
        })
        .collect()
}

#[test]
fn execution_ledger_restart_recovery_and_read_paths_are_zero_write() {
    let root = root("execution-restart");
    let key = [7_u8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    ledger.reserve().unwrap();
    ledger.publish_result(sha('8'), sha('9')).unwrap();
    ledger.require_recovery("publication-ambiguous").unwrap();
    drop(ledger);
    let mut reopened = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::RecoveryRequired { .. }
    ));
    let before = tree(&root);
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::RecoveryRequired { .. }
    ));
    assert_eq!(tree(&root), before);
    reopened.reconcile_authenticated_publication().unwrap();
    reopened.complete().unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::Terminal { .. }
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_ledger_mutation_rollback_truncation_and_special_files_fail_closed() {
    let root = root("execution-malformed");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let _ledger = FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    fs::write(root.join("execution.state"), b"{").unwrap();
    assert!(FileEvaluationExecutionLedger::open(&root, key, binding.clone()).is_err());
    fs::remove_file(root.join("execution.state")).unwrap();
    std::os::unix::fs::symlink("execution.anchor.journal", root.join("execution.state")).unwrap();
    assert!(FileEvaluationExecutionLedger::open(&root, key, binding).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_anchor_journal_recovers_state_only_rollback_and_rejects_paired_restore() {
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let state_only_root = root("execution-state-only-rollback");
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&state_only_root, key, binding.clone()).unwrap();
    let old_state = fs::read(state_only_root.join("execution.state")).unwrap();
    ledger.reserve().unwrap();
    fs::write(state_only_root.join("execution.state"), old_state).unwrap();
    let recovered =
        FileEvaluationExecutionLedger::open(&state_only_root, key, binding.clone()).unwrap();
    assert!(matches!(
        recovered.inspect().unwrap(),
        EvaluationLedgerState::Reserved
    ));

    let paired_root = root("execution-paired-rollback");
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&paired_root, key, binding.clone()).unwrap();
    let old_state = fs::read(paired_root.join("execution.state")).unwrap();
    let old_anchor = fs::read(paired_root.join("execution.anchor.journal")).unwrap();
    ledger.reserve().unwrap();
    fs::write(paired_root.join("execution.state"), old_state).unwrap();
    fs::write(paired_root.join("execution.anchor.journal"), old_anchor).unwrap();
    assert!(FileEvaluationExecutionLedger::open(&paired_root, key, binding).is_err());
    fs::remove_dir_all(state_only_root).unwrap();
    fs::remove_dir_all(paired_root).unwrap();
}

#[test]
fn execution_journal_tolerates_one_crash_tail_and_repairs_only_on_mutation() {
    let root = root("execution-partial-tail");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let ledger = FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    drop(ledger);
    let anchor_path = root.join("execution.anchor.journal");
    let stable_length = fs::metadata(&anchor_path).unwrap().len();
    let mut anchor = fs::OpenOptions::new()
        .append(true)
        .open(&anchor_path)
        .unwrap();
    anchor.write_all(&[0, 0, 0, 1]).unwrap();
    anchor.sync_all().unwrap();
    drop(anchor);

    let before = tree(&root);
    let mut reopened = FileEvaluationExecutionLedger::open(&root, key, binding.clone()).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::Initialized
    ));
    assert_eq!(tree(&root), before);
    reopened.reserve().unwrap();
    assert!(fs::metadata(&anchor_path).unwrap().len() > stable_length);
    let current = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        current.inspect().unwrap(),
        EvaluationLedgerState::Reserved
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_stale_protocol_pending_file_is_ignored_read_only_then_removed_on_mutation() {
    let root = root("execution-stale-pending");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    ledger.reserve().unwrap();
    drop(ledger);
    let pending = root.join(".execution.state.pending.999999.1");
    fs::write(&pending, b"partial-publication").unwrap();
    fs::set_permissions(&pending, fs::Permissions::from_mode(0o600)).unwrap();

    let before = tree(&root);
    let mut reopened = FileEvaluationExecutionLedger::open(&root, key, binding).unwrap();
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::Reserved
    ));
    assert_eq!(tree(&root), before);
    reopened.require_recovery("pending-reconciled").unwrap();
    assert!(!pending.exists());
    assert!(matches!(
        reopened.inspect().unwrap(),
        EvaluationLedgerState::RecoveryRequired { .. }
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn execution_lock_replacement_cannot_create_a_second_mutation_authority() {
    let root = root("execution-lock-replacement");
    let saved_lock = root.with_extension("saved-execution-lock");
    let key = [8_u8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    fs::rename(root.join("execution.lock"), &saved_lock).unwrap();
    let replacement = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("execution.lock"))
        .unwrap();
    drop(replacement);
    fs::set_permissions(
        root.join("execution.lock"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(ledger.reserve().is_err());
    assert!(FileEvaluationExecutionLedger::open(&root, key, binding).is_err());
    fs::remove_file(root.join("execution.lock")).unwrap();
    fs::rename(&saved_lock, root.join("execution.lock")).unwrap();
    fs::remove_dir_all(root).unwrap();
}
