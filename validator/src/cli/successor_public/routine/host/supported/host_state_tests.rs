use super::*;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn fixture() -> (PathBuf, PathBuf) {
    let root = std::env::var_os("CODEX_WORKTREE_SCRATCH")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("CODEX_WORKTREE_SCRATCH is required"));
    let root = fs::canonicalize(root).expect("configured worktree scratch is unavailable");
    let parent = loop {
        let candidate = root.join(format!(
            "routine-host-cache-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => break candidate,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("host cache fixture claim failed: {error}"),
        }
    };
    let home = parent.join("home");
    let target = parent.join("target");
    fs::create_dir(&target).unwrap();
    let state = home.join(".codex/state/harness-ultragoal/routine-public");
    for path in [
        home.clone(),
        home.join(".codex"),
        home.join(".codex/state"),
        home.join(".codex/state/harness-ultragoal"),
        state.clone(),
        state.join("authority"),
        state.join("adapter"),
    ] {
        fs::create_dir_all(&path).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let lock = state.join("adapter/adapter.lock");
    fs::write(&lock, LOCK_MARKER).unwrap();
    fs::set_permissions(lock, fs::Permissions::from_mode(0o600)).unwrap();
    (
        fs::canonicalize(home).unwrap(),
        fs::canonicalize(target).unwrap(),
    )
}

fn binding(target_id: &str, label: &str) -> CacheBinding {
    CacheBinding::new(
        target_id.to_owned(),
        digest(format!("source-{label}").as_bytes()),
        digest(format!("context-{label}").as_bytes()),
        digest(format!("candidate-{label}").as_bytes()),
        digest(format!("graph-{label}").as_bytes()),
        digest(format!("snapshot-{label}").as_bytes()),
        digest(format!("plan-{label}").as_bytes()),
        digest(format!("protocol-{label}").as_bytes()),
        digest(format!("request-{label}").as_bytes()),
    )
}

#[test]
fn different_binding_is_a_miss_then_each_exact_repeat_reuses() {
    let (home, target) = fixture();
    let state = HostState::open(&home, &target).unwrap();
    let first = binding(&state.target_id, "first");
    let second = binding(&state.target_id, "second");
    let first_artifact = vec![b"first-artifact".to_vec()];
    let second_artifact = vec![b"second-artifact".to_vec()];
    state.persist_reuse(first.clone(), &first_artifact).unwrap();
    assert_eq!(state.read_reuse(&first).unwrap(), Some(first_artifact));
    assert_eq!(state.read_reuse(&second).unwrap(), None);
    state
        .persist_reuse(second.clone(), &second_artifact)
        .unwrap();
    assert_eq!(state.read_reuse(&second).unwrap(), Some(second_artifact));
    assert_eq!(state.read_reuse(&first).unwrap(), None);
    fs::remove_dir_all(home.parent().unwrap()).unwrap();
}

#[test]
fn retired_cache_is_a_miss_but_unknown_or_malformed_cache_fails_closed() {
    let (home, target) = fixture();
    let state = HostState::open(&home, &target).unwrap();
    let binding = binding(&state.target_id, "schema");
    state
        .persist_reuse(binding.clone(), &[b"artifact".to_vec()])
        .unwrap();
    let cache = state.adapter.path.join(CACHE_NAME);
    let mut envelope: CacheEnvelope = serde_json::from_slice(&fs::read(&cache).unwrap()).unwrap();
    envelope.schema_version = RETIRED_CACHE_SCHEMA.to_owned();
    fs::write(&cache, serde_json::to_vec(&envelope).unwrap()).unwrap();
    assert_eq!(state.read_reuse(&binding).unwrap(), None);

    envelope.schema_version = "RoutinePublicReuseCache-v999".to_owned();
    fs::write(&cache, serde_json::to_vec(&envelope).unwrap()).unwrap();
    assert_eq!(state.read_reuse(&binding), Err(HostFailure::Invalid));
    fs::write(&cache, b"not-json").unwrap();
    assert_eq!(state.read_reuse(&binding), Err(HostFailure::Invalid));
    fs::remove_dir_all(home.parent().unwrap()).unwrap();
}
