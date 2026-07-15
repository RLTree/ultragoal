use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

#[test]
pub(crate) fn catalog_fixture_drop_and_unwind_preserve_scope_until_explicit_teardown() {
    let root = TestRoot::new("drop-inert", VALID_CATALOG);
    let path = root.path.clone();
    let sentinel = path.join("drop-sentinel");
    fs::write(&sentinel, b"catalog fixture drop must not delete this\n").unwrap();
    drop(root);
    assert_eq!(
        fs::read(&sentinel).unwrap(),
        b"catalog fixture drop must not delete this\n"
    );
    fs::remove_dir_all(&path).expect("explicit catalog fixture teardown failed");
    assert!(!path.exists());

    let retained = Arc::new(Mutex::new(None::<PathBuf>));
    let captured = Arc::clone(&retained);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let root = TestRoot::new("unwind-inert", VALID_CATALOG);
        fs::write(
            root.path.join("unwind-sentinel"),
            b"unwind keeps catalog fixture\n",
        )
        .unwrap();
        *captured.lock().unwrap() = Some(root.path.clone());
        panic!("induced catalog fixture unwind");
    }));
    assert!(result.is_err());
    let path = retained.lock().unwrap().take().unwrap();
    assert_eq!(
        fs::read(path.join("unwind-sentinel")).unwrap(),
        b"unwind keeps catalog fixture\n"
    );
    fs::remove_dir_all(&path).expect("explicit catalog unwind teardown failed");
    assert!(!path.exists());
}
