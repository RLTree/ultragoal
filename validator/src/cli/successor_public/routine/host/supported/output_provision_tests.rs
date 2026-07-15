use super::*;
use std::os::unix::fs::symlink;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Root(PathBuf);

impl Root {
    fn new(label: &str) -> Self {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .expect("output fixture manifest has no workspace parent")
            .join("target");
        fs::create_dir_all(&root).expect("output fixture target directory is unavailable");
        let root = fs::canonicalize(root).expect("output fixture target directory is unavailable");
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

    fn teardown_after_assertions(&mut self) {
        assert!(
            self.0.is_dir(),
            "output provision fixture disappeared before teardown: {}",
            self.0.display()
        );
        fs::remove_dir_all(&self.0).expect("output provision fixture teardown failed");
        assert!(
            !self.0.exists(),
            "output provision fixture teardown retained scope: {}",
            self.0.display()
        );
    }
}

#[test]
fn fresh_scopes_rollback_and_committed_scopes_repeat_safely() {
    let mut root = Root::new("fresh-repeat");
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
    root.teardown_after_assertions();
}

#[test]
fn aliases_special_objects_and_rollback_conflicts_fail_without_outside_writes() {
    let mut root = Root::new("unsafe");
    let mut outside = Root::new("outside");
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
    root.teardown_after_assertions();
    outside.teardown_after_assertions();
}

#[test]
fn fixture_drop_and_unwind_preserve_output_scope_until_explicit_teardown() {
    let root = Root::new("drop-inert");
    let path = root.0.clone();
    fs::write(
        path.join("drop-sentinel"),
        b"drop must not erase output scope\n",
    )
    .unwrap();
    drop(root);
    assert_eq!(
        fs::read(path.join("drop-sentinel")).unwrap(),
        b"drop must not erase output scope\n"
    );
    fs::remove_dir_all(&path).expect("explicit output fixture teardown failed");
    assert!(!path.exists());

    let result = std::panic::catch_unwind(|| {
        let root = Root::new("unwind-inert");
        let path = root.0.clone();
        fs::write(
            path.join("unwind-sentinel"),
            b"unwind must not erase output scope\n",
        )
        .unwrap();
        std::panic::panic_any(path);
    });
    let path = *result.unwrap_err().downcast::<PathBuf>().unwrap();
    assert_eq!(
        fs::read(path.join("unwind-sentinel")).unwrap(),
        b"unwind must not erase output scope\n"
    );
    fs::remove_dir_all(&path).expect("explicit unwind fixture teardown failed");
    assert!(!path.exists());
}
