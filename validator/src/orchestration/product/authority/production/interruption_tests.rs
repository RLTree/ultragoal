use super::store::Store;
use super::{Ledger, LedgerState, ProductError};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

const CHILD_ROOT: &str = "ULTRAGOAL_AUTHORITY_SETTLEMENT_CHILD_ROOT";
const ACTOR: &str = "/root";

#[test]
fn nonempty_fresh_process_reopen_refuses_before_operation_without_external_custody() {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let root = PathBuf::from(root);
        let (store, _) = Store::open_existing(&root, ACTOR).unwrap();
        assert!(matches!(
            Ledger::open(store),
            Err(ProductError::AuthorityCheckpointRequired)
        ));
        return;
    }

    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "orchestration-authority-settlement-death-{}-{}",
        std::process::id(),
        module_path!().replace("::", "-")
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let permit = format!("sha256:{}", "a".repeat(64));
    let (store, _) = Store::open_or_initialize(&root, ACTOR).unwrap();
    let ledger = Ledger::open(store).unwrap();
    ledger.issue(&permit, &permit).unwrap();
    ledger.reserve_for_test(&permit).unwrap();
    let test_name = format!(
        "{}::nonempty_fresh_process_reopen_refuses_before_operation_without_external_custody",
        module_path!()
            .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
            .unwrap_or(module_path!())
    );
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &test_name, "--nocapture"])
        .env(CHILD_ROOT, &root)
        .status()
        .unwrap();
    assert!(status.success());
    let marker = root.with_extension("operation-observed");
    assert!(!marker.exists());
    assert_eq!(ledger.state(&permit).unwrap(), Some(LedgerState::Reserved));
    ledger.reconcile(&permit, LedgerState::Committed).unwrap();
    assert_eq!(ledger.state(&permit).unwrap(), Some(LedgerState::Committed));
    fs::remove_dir_all(root).unwrap();
}
