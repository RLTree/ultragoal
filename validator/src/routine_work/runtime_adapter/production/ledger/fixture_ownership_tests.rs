use super::*;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

#[test]
fn ledger_fixture_drop_and_unwind_preserve_scope_until_explicit_teardown() {
    let root = TestRoot::new("drop-inert");
    let path = root.parent.clone();
    fs::write(
        path.join("drop-sentinel"),
        b"drop must not erase ledger fixture\n",
    )
    .unwrap();
    drop(root);
    assert_eq!(
        fs::read(path.join("drop-sentinel")).unwrap(),
        b"drop must not erase ledger fixture\n"
    );
    fs::remove_dir_all(&path).expect("explicit ledger fixture teardown failed");
    assert!(!path.exists());

    let retained = Arc::new(Mutex::new(None));
    let retained_for_panic = Arc::clone(&retained);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let root = TestRoot::new("unwind-inert");
        fs::write(
            root.parent.join("unwind-sentinel"),
            b"unwind must not erase ledger fixture\n",
        )
        .unwrap();
        *retained_for_panic.lock().unwrap() = Some(root.parent.clone());
        panic!("induced ledger fixture unwind");
    }));
    assert!(result.is_err());
    let path = retained.lock().unwrap().take().unwrap();
    assert_eq!(
        fs::read(path.join("unwind-sentinel")).unwrap(),
        b"unwind must not erase ledger fixture\n"
    );
    fs::remove_dir_all(&path).expect("explicit unwind ledger teardown failed");
    assert!(!path.exists());
}
