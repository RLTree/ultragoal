use super::scenario::{Fixture, pass_node, prefix_route};
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn fixture(label: &str) -> Fixture {
    Fixture::new(
        label,
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    )
}

fn teardown_after_observation(path: &PathBuf) {
    fs::remove_dir_all(path).expect("explicit lifecycle control teardown failed");
    assert!(!path.exists(), "lifecycle control retained fixture scope");
}

#[test]
fn ordinary_fixture_drop_preserves_scope_until_explicit_teardown() {
    let fixture = fixture("drop-inert");
    let path = fixture.container.clone();
    let sentinel = path.join("drop-sentinel");
    fs::write(&sentinel, b"fixture drop must not delete this\n").unwrap();
    drop(fixture);
    assert_eq!(
        fs::read(&sentinel).unwrap(),
        b"fixture drop must not delete this\n"
    );
    teardown_after_observation(&path);
}

#[test]
fn unwinding_fixture_drop_preserves_scope_until_explicit_teardown() {
    let retained = Arc::new(Mutex::new(None::<PathBuf>));
    let retained_for_panic = Arc::clone(&retained);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let fixture = fixture("unwind-inert");
        let path = fixture.container.clone();
        fs::write(
            path.join("unwind-sentinel"),
            b"unwind must not clean fixture\n",
        )
        .unwrap();
        *retained_for_panic.lock().unwrap() = Some(path);
        panic!("induced fixture unwind");
    }));
    assert!(result.is_err());
    let path = retained.lock().unwrap().take().unwrap();
    assert_eq!(
        fs::read(path.join("unwind-sentinel")).unwrap(),
        b"unwind must not clean fixture\n"
    );
    teardown_after_observation(&path);
}
