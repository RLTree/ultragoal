use super::{ClosureError, checked_relative, checked_root, hash_stable};
use std::fs;
use std::os::unix::fs::symlink;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn source_ancestor_symlink_is_never_followed() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("source-closure-{suffix}"));
    let outside = root.with_extension("outside");
    fs::create_dir_all(&root).expect("root");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(outside.join("dep-info"), b"secret").expect("outside dep-info");
    symlink(&outside, root.join("target")).expect("ancestor symlink");

    let root_descriptor = checked_root(&root).expect("checked root");
    let relative = checked_relative("target/dep-info").expect("relative path");
    assert_eq!(
        hash_stable(&root_descriptor, &relative).expect_err("ancestor symlink must fail closed"),
        ClosureError::SpecialFile
    );

    fs::remove_dir_all(&root).expect("cleanup root");
    fs::remove_dir_all(outside).expect("cleanup outside");
}
