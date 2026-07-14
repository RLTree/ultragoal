use super::*;
use std::os::unix::fs::symlink;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Root(PathBuf);

impl Root {
    fn new(label: &str) -> Self {
        let root = std::env::var_os("CODEX_WORKTREE_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(|| panic!("CODEX_WORKTREE_SCRATCH is required"));
        let root = fs::canonicalize(root).expect("configured worktree scratch is unavailable");
        let path = loop {
            let candidate = root.join(format!(
                "routine-output-provision-{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("output provision fixture claim failed: {error}"),
            }
        };
        Self(fs::canonicalize(path).unwrap())
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn fresh_scopes_rollback_and_committed_scopes_repeat_safely() {
    let root = Root::new("fresh-repeat");
    let nodes = vec!["compile".to_owned(), "lint".to_owned()];
    let provision = OutputProvision::create(&root.0, &nodes).unwrap();
    assert!(root.0.join("target/routine/compile").is_dir());
    assert!(root.0.join("target/routine/lint").is_dir());
    provision.rollback().unwrap();
    assert!(!root.0.join("target").exists());

    OutputProvision::create(&root.0, &nodes).unwrap().commit();
    OutputProvision::create(&root.0, &nodes)
        .unwrap()
        .rollback()
        .unwrap();
    assert!(root.0.join("target/routine/compile").is_dir());
}

#[test]
fn aliases_special_objects_and_rollback_conflicts_fail_without_outside_writes() {
    let root = Root::new("unsafe");
    let outside = Root::new("outside");
    fs::create_dir(root.0.join("target")).unwrap();
    symlink(&outside.0, root.0.join("target/routine")).unwrap();
    assert!(OutputProvision::create(&root.0, &["compile".to_owned()]).is_err());
    assert!(fs::read_dir(&outside.0).unwrap().next().is_none());

    fs::remove_file(root.0.join("target/routine")).unwrap();
    fs::write(root.0.join("target/routine"), b"special substitute").unwrap();
    assert!(OutputProvision::create(&root.0, &["compile".to_owned()]).is_err());
    fs::remove_file(root.0.join("target/routine")).unwrap();

    let provision = OutputProvision::create(&root.0, &["compile".to_owned()]).unwrap();
    fs::write(root.0.join("target/routine/compile/foreign"), b"foreign").unwrap();
    assert_eq!(provision.rollback(), Err(HostFailure::Persistence));
}
