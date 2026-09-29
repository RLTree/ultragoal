use super::super::*;
use super::custody::{execution_binding, root};
use super::recovery_setup::{preparation_spec, sha, RefusingBridge};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

#[test]
fn preparation_issues_private_authority_and_derives_material_binding() {
    let input_root = root("preparation-input");
    let ledger_root = root("preparation-ledger");
    let spec = preparation_spec(&input_root);
    let request = super::super::runtime::ProductionExecutionRequest::new(
        &spec,
        &input_root,
        &ledger_root,
        [9; 32],
        sha('1'),
        sha('8'),
        RuntimeConfiguration::all_unknown(),
    );
    let error = request.execute(&mut RefusingBridge).unwrap_err();
    assert_eq!(error.code(), "evaluation-test-bridge-refused");
    assert!(fs::read_dir(&ledger_root).unwrap().next().is_some());
    fs::remove_dir_all(input_root).unwrap();
    fs::remove_dir_all(ledger_root).unwrap();
}

#[test]
fn execution_symlink_and_lock_replacement_refuse_second_authority() {
    let root = root("descriptor-refusal");
    let key = [8; 32];
    let binding = execution_binding('b', '1');
    let mut ledger =
        FileEvaluationExecutionLedger::initialize(&root, key, binding.clone()).unwrap();
    let saved = root.with_extension("saved-lock");
    fs::rename(root.join("execution.lock"), &saved).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("execution.lock"))
        .unwrap();
    fs::set_permissions(
        root.join("execution.lock"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(ledger.reserve_outcome(&sha('7')).is_err());
    assert!(FileEvaluationExecutionLedger::open(&root, key, binding.clone()).is_err());
    fs::remove_file(root.join("execution.lock")).unwrap();
    fs::rename(&saved, root.join("execution.lock")).unwrap();
    fs::remove_file(root.join("execution.state")).unwrap();
    symlink("execution.anchor.journal", root.join("execution.state")).unwrap();
    assert!(FileEvaluationExecutionLedger::open(&root, key, binding).is_err());
    fs::remove_dir_all(root).unwrap();
}
