use super::scenario::*;
use std::ffi::CString;
use std::fs::{self, hard_link};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, symlink};

fn assert_path_boundary(repository: &Repository, finding: &SelectedFinding) {
    let (_, diagnostic) =
        assert_diagnostic_zero_write(repository, &["--json", "observe", "query"], 4);
    assert_eq!(
        diagnostic["diagnostic_id"],
        "successor_runtime_observability_unavailable"
    );
    let (_, diagnosis) = assert_payload_zero_write(
        repository,
        &["--json", "diagnose", "--finding", &finding.finding_id],
        &[],
        &[1],
        "ProductStateDiagnose-v1",
    );
    let failure = &diagnosis["observability"]["read_failure"];
    assert_eq!(failure["schema_version"], "LocalStoreReadFailure-v1");
    assert_eq!(failure["stage"], "open");
    assert_eq!(failure["class"], "path-boundary");
    assert_eq!(
        failure["diagnostic_code"],
        "observe-local-read:path-boundary"
    );
    assert_eq!(diagnosis["observability"]["store_status"], "unavailable");
    assert_eq!(diagnosis["observability"]["claim_effect"], "none");
}

#[test]
fn ancestor_symlink_is_not_followed_or_disclosed() {
    let repository = Repository::new("ancestor-symlink", true, true);
    let _ = binding(&repository);
    let finding = selected_finding(&repository);
    let outside = repository.outside("outside-spool");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("private.txt"), PRIVATE_TOKEN).unwrap();
    fs::create_dir_all(repository.root().join("validation_artifacts/observability")).unwrap();
    symlink(
        &outside,
        repository
            .root()
            .join("validation_artifacts/observability/spool"),
    )
    .unwrap();
    let outside_before = fs::read(outside.join("private.txt")).unwrap();
    assert_path_boundary(&repository, &finding);
    assert_eq!(
        fs::read(outside.join("private.txt")).unwrap(),
        outside_before
    );
    repository.teardown();
}

#[test]
fn fifo_and_multiply_linked_leaf_fail_closed_without_interpretation() {
    let fifo = Repository::new("fifo-leaf", true, false);
    let fifo_binding = binding(&fifo);
    let fifo_finding = selected_finding(&fifo);
    fs::create_dir_all(fifo.store_path(&fifo_binding).parent().unwrap()).unwrap();
    let name = CString::new(fifo.store_path(&fifo_binding).as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert_path_boundary(&fifo, &fifo_finding);

    let linked = Repository::new("hardlink-leaf", true, false);
    let binding = binding(&linked);
    let linked_finding = selected_finding(&linked);
    let store = open_store(&linked, &binding);
    assert!(
        store
            .append(&event(&binding, "linked-event", 1, "check.run", "fail"))
            .unwrap()
    );
    let outside = linked.outside("outside-hardlink.jsonl");
    hard_link(linked.store_path(&binding), &outside).unwrap();
    assert_eq!(
        fs::metadata(linked.store_path(&binding)).unwrap().nlink(),
        2
    );
    let outside_before = fs::read(&outside).unwrap();
    assert_path_boundary(&linked, &linked_finding);
    assert_eq!(fs::read(&outside).unwrap(), outside_before);
    fifo.teardown();
    linked.teardown();
}
