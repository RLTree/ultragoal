use super::read_session::{set_test_pause_after_first_read, set_test_pause_before_open};
use super::tests::{Repo, run_git};
use super::{BuildRequest, ContextError, LiveContext};
use std::fs;
use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[cfg(unix)]
#[test]
fn read_bounded_rejects_in_place_mutate_and_restore() {
    let repo = Repo::new("bounded-read-mutate");
    let original = vec![b'a'; 64 * 1024];
    let mutated = vec![b'b'; 64 * 1024];
    fs::write(repo.root.join("tracked.txt"), &original).unwrap();
    run_git(&repo.root, &["add", "tracked.txt"]);
    run_git(&repo.root, &["commit", "-q", "-m", "large source"]);
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let session = context.begin_read_session().unwrap();
    let target = session.root().join("tracked.txt");
    let target_for_mutation = target.clone();
    set_test_pause_after_first_read(150);
    let mutation = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(30));
        fs::write(&target_for_mutation, &mutated).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(30));
        fs::write(&target_for_mutation, &original).unwrap();
    });
    let result = session.read_bounded(&target, 128 * 1024);
    mutation.join().unwrap();
    assert!(matches!(result, Err(ContextError::ConcurrentMutation(_))));
}

#[cfg(unix)]
#[test]
fn session_revalidation_rejects_between_read_swap_and_restore() {
    let repo = Repo::new("between-read-swap");
    let original = vec![b'b'; 32 * 1024];
    let transient = vec![b'a'; 32 * 1024];
    fs::write(repo.root.join("tracked.txt"), &original).unwrap();
    run_git(&repo.root, &["add", "tracked.txt"]);
    run_git(&repo.root, &["commit", "-q", "-m", "stable source"]);
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let session = context.begin_read_session().unwrap();
    let target = session.root().join("tracked.txt");
    fs::write(&target, &transient).unwrap();
    assert_eq!(session.read_bounded(&target, 64 * 1024).unwrap(), transient);
    fs::write(&target, &original).unwrap();
    assert!(matches!(
        session.revalidate(),
        Err(ContextError::ConcurrentMutation(_))
    ));
}

#[cfg(unix)]
#[test]
fn session_revalidation_rejects_hidden_entry_restore() {
    let repo = Repo::new("hidden-entry-restore");
    let collection = repo.root.join("generated");
    let holding = repo.root.join("holding");
    fs::create_dir_all(&collection).unwrap();
    fs::create_dir_all(&holding).unwrap();
    fs::write(collection.join("hidden.json"), b"{}\n").unwrap();
    run_git(&repo.root, &["add", "generated/hidden.json"]);
    run_git(&repo.root, &["commit", "-q", "-m", "generated source"]);
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let session = context.begin_read_session().unwrap();
    fs::rename(collection.join("hidden.json"), holding.join("hidden.json")).unwrap();
    session.pin_directory(&collection).unwrap();
    fs::rename(holding.join("hidden.json"), collection.join("hidden.json")).unwrap();
    assert!(matches!(
        session.revalidate(),
        Err(ContextError::ConcurrentMutation(_))
    ));
}

#[cfg(unix)]
#[test]
fn session_revalidation_rejects_hidden_collection_restore() {
    let repo = Repo::new("hidden-collection-restore");
    let collection = repo.root.join("examples/generated");
    fs::create_dir_all(&collection).unwrap();
    fs::write(collection.join("hidden.json"), b"{}\n").unwrap();
    run_git(&repo.root, &["add", "examples/generated/hidden.json"]);
    run_git(&repo.root, &["commit", "-q", "-m", "generated collection"]);
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let session = context.begin_read_session().unwrap();
    let outside = repo.root.with_file_name(format!(
        "{}-hidden-collection",
        repo.root.file_name().unwrap().to_string_lossy()
    ));
    fs::rename(&collection, &outside).unwrap();
    session.observe_presence_parent(&collection).unwrap();
    assert!(!collection.exists());
    fs::rename(&outside, &collection).unwrap();
    assert!(matches!(
        session.revalidate(),
        Err(ContextError::ConcurrentMutation(_))
    ));
}

#[test]
fn failed_bounded_read_retains_transient_size_observation() {
    let repo = Repo::new("bounded-read-size-restore");
    let original = fs::read(repo.root.join("tracked.txt")).unwrap();
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let session = context.begin_read_session().unwrap();
    let target = session.root().join("tracked.txt");
    fs::write(&target, vec![b'x'; 2 * 1024 * 1024 + 1]).unwrap();
    assert!(session.read_bounded(&target, 2 * 1024 * 1024).is_err());
    fs::write(&target, original).unwrap();
    assert!(matches!(
        session.revalidate(),
        Err(ContextError::ConcurrentMutation(_))
    ));
}

#[test]
fn repeated_rejected_reads_exhaust_aggregate_byte_budget() {
    use super::read_session::MAX_READ_SESSION_BYTES;
    let repo = Repo::new("bounded-read-aggregate-rejections");
    let target = repo.root.join("rejected.bin");
    fs::write(&target, vec![b'x'; 32 * 1024]).unwrap();
    let later = (0..8)
        .map(|index| repo.root.join(format!("post-exhaustion-{index}.bin")))
        .collect::<Vec<_>>();
    for path in &later {
        fs::write(path, b"must not be opened").unwrap();
    }
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let session = context.begin_read_session().unwrap();
    session.charge(MAX_READ_SESSION_BYTES - 32 * 1024).unwrap();
    assert!(session.read_bounded(&target, 1).is_err());
    assert!(session.read_bounded(&target, 1).is_err());
    assert_eq!(session.observation_count(), 1);
    for path in later {
        let error = session.read_bounded(&path, 1).unwrap_err();
        assert!(error.to_string().contains("read session exceeds"));
    }
    assert_eq!(session.observation_count(), 1);
}

#[cfg(unix)]
#[test]
fn read_session_rejects_parent_swap_during_descriptor_open() {
    let repo = Repo::new("read-open-race");
    let safe = repo.root.join("safe");
    fs::create_dir(&safe).unwrap();
    fs::write(safe.join("source.txt"), b"candidate source\n").unwrap();
    run_git(&repo.root, &["add", "safe/source.txt"]);
    run_git(&repo.root, &["commit", "-q", "-m", "safe source"]);
    let outside = std::env::temp_dir().join(format!(
        "ultragoal-context-read-race-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("source.txt"), b"outside source\n").unwrap();
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let session = context.begin_read_session().unwrap();
    let safe_for_swap = safe.clone();
    let outside_for_swap = outside.clone();
    set_test_pause_before_open(150);
    let swap = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(30));
        fs::rename(
            &safe_for_swap,
            safe_for_swap.with_file_name("safe-original"),
        )
        .unwrap();
        std::os::unix::fs::symlink(&outside_for_swap, &safe_for_swap).unwrap();
    });
    let result = session.open_regular(&repo.root.join("safe/source.txt"));
    swap.join().unwrap();
    assert!(result.is_err());
    let _ = fs::remove_dir_all(outside);
}

#[cfg(unix)]
#[test]
fn read_session_rejects_whole_worktree_replacement() {
    let repo = Repo::new("read-root-replacement");
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let session = context.begin_read_session().unwrap();
    let moved = repo.root.with_file_name(format!(
        "ultragoal-context-read-moved-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::rename(&repo.root, &moved).unwrap();
    fs::create_dir(&repo.root).unwrap();
    fs::write(repo.root.join("tracked.txt"), b"replacement source\n").unwrap();
    let result = session.open_regular(&repo.root.join("tracked.txt"));
    match result {
        Ok(mut file) => {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes, b"inside\n");
        }
        Err(ContextError::PathDenied(_) | ContextError::ConcurrentMutation(_)) => {}
        Err(error) => panic!("unexpected read error: {error}"),
    }
    assert!(session.revalidate().is_err());
    fs::remove_dir_all(&repo.root).unwrap();
    fs::rename(moved, &repo.root).unwrap();
}

#[test]
fn read_session_enforces_aggregate_entry_and_byte_budgets() {
    let repo = Repo::new("read-budgets");
    let context = LiveContext::build(BuildRequest::new(&repo.root)).unwrap();
    let entries = context.begin_read_session().unwrap();
    for _ in 0..250_000 {
        entries.charge_entry().unwrap();
    }
    assert!(entries.charge_entry().is_err());

    let bytes = context.begin_read_session().unwrap();
    for _ in 0..512 {
        bytes.charge(1024 * 1024).unwrap();
    }
    assert!(bytes.charge(1).is_err());
}
