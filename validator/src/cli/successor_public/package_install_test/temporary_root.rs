use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

pub(super) fn create() -> Result<PathBuf, &'static str> {
    let parent = crate::distribution::filesystem::canonical_temporary_parent()
        .map_err(|_| "temporary parent unavailable")?;
    for _ in 0..16 {
        let path = parent.join(format!(
            "hul-distribution-host-effect-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(0o700);
        match builder.create(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("disposable isolated host creation failed"),
        }
    }
    Err("unique disposable isolated host unavailable")
}
