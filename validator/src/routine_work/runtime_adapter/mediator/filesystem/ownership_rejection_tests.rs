use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn root(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "hul-output-confinement-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(path.join("target/routine/compile")).unwrap();
    fs::canonicalize(path).unwrap()
}

#[test]
fn stale_files_cannot_acquire_current_execution_provenance() {
    let path = root("stale");
    fs::write(path.join("target/routine/compile/stale"), b"stale").unwrap();
    let anchor = RootAnchor::open(&path).unwrap();
    let result = OutputConfinement::prepare(
        &anchor,
        &[RepoPath::parse("target/routine/compile").unwrap()],
        1024,
    );
    let error = match result {
        Ok(_) => panic!("stale output was accepted"),
        Err(error) => error,
    };
    assert_eq!(error.cause(), "mediator-output-scope-not-empty");
    fs::remove_dir_all(path).unwrap();
}

#[test]
fn create_delete_restore_is_not_an_empty_owned_delta() {
    let path = root("restore");
    let anchor = RootAnchor::open(&path).unwrap();
    let outputs = OutputConfinement::prepare(
        &anchor,
        &[RepoPath::parse("target/routine/compile").unwrap()],
        1024,
    )
    .unwrap();
    let transient = path.join("target/routine/compile/transient");
    fs::write(&transient, b"transient").unwrap();
    fs::remove_file(transient).unwrap();
    assert_eq!(
        outputs.capture_owned_delta().unwrap_err().id(),
        RoutineErrorId::ConcurrentMutation
    );
    fs::remove_dir_all(path).unwrap();
}
