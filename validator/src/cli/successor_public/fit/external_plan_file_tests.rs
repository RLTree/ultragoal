use super::external_plan_file::read_immutable_plan;
use std::fs;
use std::os::unix::fs::symlink;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "hul-fit-plan-input-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> std::path::PathBuf {
        self.root.join(name)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn external_plan_reader_accepts_one_owned_regular_file_without_mutation() {
    let fixture = Fixture::new();
    let path = fixture.path("plan.json");
    fs::write(&path, b"{\"plan\":\"bound\"}").unwrap();
    let before = fs::metadata(&path).unwrap();
    assert_eq!(
        read_immutable_plan(&path, 1024).unwrap(),
        b"{\"plan\":\"bound\"}"
    );
    let after = fs::metadata(&path).unwrap();
    assert_eq!(before.len(), after.len());
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
}

#[test]
fn external_plan_reader_rejects_aliases_nonfiles_and_oversize_inputs() {
    let fixture = Fixture::new();
    let original = fixture.path("original.json");
    fs::write(&original, b"bound").unwrap();

    let hardlink = fixture.path("hardlink.json");
    fs::hard_link(&original, &hardlink).unwrap();
    assert!(read_immutable_plan(&original, 1024).is_err());
    assert!(read_immutable_plan(&hardlink, 1024).is_err());

    let symlink_path = fixture.path("symlink.json");
    symlink(&original, &symlink_path).unwrap();
    assert!(read_immutable_plan(&symlink_path, 1024).is_err());

    let directory = fixture.path("directory");
    fs::create_dir(&directory).unwrap();
    assert!(read_immutable_plan(&directory, 1024).is_err());

    let oversized = fixture.path("oversized.json");
    fs::write(&oversized, [0_u8; 16]).unwrap();
    assert!(read_immutable_plan(&oversized, 15).is_err());
}
