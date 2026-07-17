use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

pub(crate) fn create(label: &str) -> io::Result<PathBuf> {
    validate_label(label)?;
    let base = std::env::var_os("CARGO_TARGET_TMPDIR")
        .or_else(|| std::env::var_os("TMPDIR"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    if !base.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime test root must be absolute",
        ));
    }
    let metadata = fs::symlink_metadata(&base)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime test root must be a real directory",
        ));
    }
    for _ in 0..32 {
        let serial = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let path = base.join(format!(
            "ultragoal-orchestration-{label}-{}-{serial}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate an isolated runtime test root",
    ))
}

fn validate_label(label: &str) -> io::Result<()> {
    let valid = !label.is_empty()
        && label.len() <= 96
        && label
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime test-root label is invalid",
        ))
    }
}
