use super::recursive_snapshot::{ZeroWriteCaptureError, capture};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "ultragoal-strict-zero-write-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create strict zero-write fixture");
        Self { root }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn recursive_guard_detects_content_creation_and_removal() {
    let fixture = Fixture::new("content");
    fs::create_dir(fixture.root.join("scripts")).unwrap();
    fs::write(fixture.root.join("scripts/check.py"), b"before\n").unwrap();
    let before = capture(&fixture.root).unwrap();

    fs::write(fixture.root.join("scripts/check.py"), b"after\n").unwrap();
    let changed = capture(&fixture.root).unwrap();
    assert_ne!(changed, before);

    fs::remove_file(fixture.root.join("scripts/check.py")).unwrap();
    assert_ne!(capture(&fixture.root).unwrap(), changed);
}

#[test]
fn recursive_guard_includes_claim_bearing_validation_artifacts() {
    let fixture = Fixture::new("claim-artifacts");
    let review = fixture.root.join("validation_artifacts/review");
    fs::create_dir_all(&review).unwrap();
    let before = capture(&fixture.root).unwrap();

    fs::write(review.join("hidden-write.json"), b"{}\n").unwrap();
    assert_ne!(capture(&fixture.root).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn recursive_guard_detects_mode_only_drift() {
    let fixture = Fixture::new("mode");
    let path = fixture.root.join("check");
    fs::write(&path, b"stable\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let before = capture(&fixture.root).unwrap();

    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    assert_ne!(capture(&fixture.root).unwrap(), before);
}

#[test]
fn hardlinked_regular_input_fails_closed() {
    let fixture = Fixture::new("hardlink");
    let first = fixture.root.join("first");
    fs::write(&first, b"same object\n").unwrap();
    fs::hard_link(&first, fixture.root.join("second")).unwrap();

    assert_eq!(
        capture(&fixture.root).unwrap_err(),
        ZeroWriteCaptureError::ConcurrentMutation
    );
}
