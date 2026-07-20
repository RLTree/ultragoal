use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

pub(super) fn create() -> Result<PathBuf, &'static str> {
    let parent = std::env::var_os("CODEX_WORKTREE_TMP")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .canonicalize()
        .map_err(|_| "temporary parent unavailable")?;
    for _ in 0..16 {
        let path = parent.join(format!(
            "hul-distribution-install-test-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("disposable isolated host creation failed"),
        }
    }
    Err("unique disposable isolated host unavailable")
}
