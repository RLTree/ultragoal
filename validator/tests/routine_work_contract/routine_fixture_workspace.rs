use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_WORKSPACE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn claim_routine_fixture_root(label: &str) -> PathBuf {
    let worktree = std::env::var_os("CODEX_WORKTREE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("CODEX_WORKTREE_ROOT is required"));
    let worktree = fs::canonicalize(worktree).expect("configured worktree root unavailable");
    let parent = worktree.join("target/routine-work-contract-fixtures");
    fs::create_dir_all(&parent).unwrap();
    loop {
        let sequence = NEXT_WORKSPACE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            "hul-routine-{label}-{}-{sequence}",
            std::process::id()
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => return candidate,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("routine fixture claim failed: {error}"),
        }
    }
}
