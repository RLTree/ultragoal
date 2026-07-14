use std::fs;
use std::os::unix::fs::symlink;
use std::os::unix::net::UnixListener;

use super::configured_path_alias::ConfiguredPathAlias;
use super::routine_plan_fixture::repo_path;
use super::routine_work::{
    RoutineErrorId, set_test_output_capture_hook, validate_output_confinement_after,
    validate_read_confinement_after_bind,
};
use super::scenario::TempRepo;

fn cause<T>(result: Result<T, super::routine_work::RoutineError>) -> &'static str {
    result.err().expect("unsafe object must refuse").cause()
}

fn root(label: &str) -> (TempRepo, std::path::PathBuf) {
    let repo = TempRepo::new(label);
    fs::create_dir_all(repo.root().join("target/routine")).unwrap();
    let root = fs::canonicalize(repo.root()).unwrap();
    (repo, root)
}

fn short_alias(repo: &TempRepo) -> ConfiguredPathAlias {
    ConfiguredPathAlias::claim("routine-filesystem", repo.root())
}

#[test]
fn output_symlink_hardlink_fifo_socket_and_stale_files_refuse_exact_capture() {
    let (repo, root) = root("output-object-refusals");
    let scope = repo_path("target/routine");
    let outside = repo.root().join("outside");
    fs::write(&outside, b"outside").unwrap();

    symlink(&outside, repo.root().join("target/routine/link")).unwrap();
    assert_eq!(
        cause(validate_output_confinement_after(
            &root,
            &[scope.clone()],
            1024,
            || {}
        )),
        "mediator-output-object-unsafe"
    );
    fs::remove_file(repo.root().join("target/routine/link")).unwrap();

    fs::write(repo.root().join("target/routine/first"), b"same").unwrap();
    fs::hard_link(
        repo.root().join("target/routine/first"),
        repo.root().join("target/routine/second"),
    )
    .unwrap();
    assert_eq!(
        cause(validate_output_confinement_after(
            &root,
            &[scope.clone()],
            1024,
            || {}
        )),
        "mediator-output-hardlink-refused"
    );
    fs::remove_file(repo.root().join("target/routine/first")).unwrap();
    fs::remove_file(repo.root().join("target/routine/second")).unwrap();

    let fifo = repo.root().join("target/routine/fifo");
    let fifo_c = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
    assert_eq!(
        cause(validate_output_confinement_after(
            &root,
            &[scope.clone()],
            1024,
            || {}
        )),
        "mediator-output-object-unsafe"
    );
    fs::remove_file(&fifo).unwrap();

    let alias = short_alias(&repo);
    let socket = repo.root().join("target/routine/socket");
    let listener =
        UnixListener::bind(alias.child_from_current_dir("target/routine/socket")).unwrap();
    assert_eq!(
        cause(validate_output_confinement_after(
            &root,
            &[scope.clone()],
            1024,
            || {}
        )),
        "mediator-output-object-unsafe"
    );
    drop(listener);
    fs::remove_file(&socket).unwrap();
    drop(alias);

    fs::write(repo.root().join("target/routine/stale"), b"stale").unwrap();
    assert_eq!(
        cause(validate_output_confinement_after(
            &root,
            &[scope],
            1024,
            || {}
        )),
        "mediator-output-scope-not-empty"
    );
}

#[test]
fn output_nested_swap_and_create_delete_restore_refuse_final_validation() {
    let (repo, root) = root("output-races");
    let scope = repo_path("target/routine");
    fs::create_dir(repo.root().join("target/routine/nested")).unwrap();
    let current = repo.root().join("target/routine/nested");
    let held = repo.root().join("target/routine/nested-held");
    set_test_output_capture_hook({
        let current = current.clone();
        let held = held.clone();
        move || {
            fs::rename(&current, &held).unwrap();
            fs::create_dir(&current).unwrap();
        }
    });
    assert_eq!(
        validate_output_confinement_after(&root, &[scope.clone()], 1024, || {})
            .unwrap_err()
            .id(),
        RoutineErrorId::ConcurrentMutation
    );
    fs::remove_dir(&current).unwrap();
    fs::rename(&held, &current).unwrap();

    assert_eq!(
        validate_output_confinement_after(&root, &[scope], 1024, || {
            let transient = repo.root().join("target/routine/transient");
            fs::write(&transient, b"transient").unwrap();
            fs::remove_file(transient).unwrap();
        })
        .unwrap_err()
        .id(),
        RoutineErrorId::ConcurrentMutation
    );
}

#[test]
fn read_symlink_hardlink_fifo_socket_ancestor_swap_and_restore_refuse() {
    let (repo, root) = root("read-object-refusals");
    let source = repo.root().join("src/lib.rs");
    symlink("lib.rs", repo.root().join("src/link.rs")).unwrap();
    assert!(
        validate_read_confinement_after_bind(&root, &[repo_path("src/link.rs")], || {}).is_err()
    );
    fs::remove_file(repo.root().join("src/link.rs")).unwrap();

    fs::hard_link(&source, repo.root().join("src/hard.rs")).unwrap();
    assert!(
        validate_read_confinement_after_bind(&root, &[repo_path("src/hard.rs")], || {}).is_err()
    );
    fs::remove_file(repo.root().join("src/hard.rs")).unwrap();

    let fifo = repo.root().join("src/fifo");
    let fifo_c = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
    assert!(validate_read_confinement_after_bind(&root, &[repo_path("src/fifo")], || {}).is_err());
    fs::remove_file(&fifo).unwrap();

    let alias = short_alias(&repo);
    let socket = repo.root().join("src/socket");
    let listener = UnixListener::bind(alias.child_from_current_dir("src/socket")).unwrap();
    assert!(
        validate_read_confinement_after_bind(&root, &[repo_path("src/socket")], || {}).is_err()
    );
    drop(listener);
    fs::remove_file(&socket).unwrap();
    drop(alias);

    let original = fs::read(&source).unwrap();
    assert_eq!(
        validate_read_confinement_after_bind(&root, &[repo_path("src/lib.rs")], || {
            fs::write(&source, b"mutated\n").unwrap();
            fs::write(&source, &original).unwrap();
        })
        .unwrap_err()
        .id(),
        RoutineErrorId::ConcurrentMutation
    );

    let held = repo.root().join("src-held");
    assert_eq!(
        validate_read_confinement_after_bind(&root, &[repo_path("src/lib.rs")], || {
            fs::rename(repo.root().join("src"), &held).unwrap();
            fs::create_dir(repo.root().join("src")).unwrap();
            fs::write(repo.root().join("src/lib.rs"), &original).unwrap();
        })
        .unwrap_err()
        .id(),
        RoutineErrorId::ConcurrentMutation
    );
}
