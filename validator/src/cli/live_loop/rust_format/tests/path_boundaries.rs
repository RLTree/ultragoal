use super::super::is_package_owned_rustfmt_input;
use super::git_workspace::{cleanup, git_fixture, write_text};
use std::path::Path;

#[test]
fn changed_rust_input_must_be_regular_package_file() {
    let root = git_fixture("rust-format-source-boundary");
    write_text(
        &root.join("validator/src/lib.rs"),
        "pub fn answer() -> i32 { 42 }\n",
    );

    assert!(is_package_owned_rustfmt_input(
        &root,
        Path::new("validator/src/lib.rs")
    ));
    assert!(!is_package_owned_rustfmt_input(
        &root,
        Path::new("../outside.rs")
    ));
    assert!(!is_package_owned_rustfmt_input(
        &root,
        Path::new("/tmp/outside.rs")
    ));
    cleanup(root);
}

#[cfg(unix)]
#[test]
fn changed_rust_input_rejects_symlinks_before_rustfmt() {
    let root = git_fixture("rust-format-symlink-boundary");
    let outside = root.with_extension("outside.rs");
    write_text(&outside, "pub fn outside() -> i32 { 42 }\n");
    std::fs::create_dir_all(root.join("validator/src")).expect("source dir");
    std::os::unix::fs::symlink(&outside, root.join("validator/src/lib.rs"))
        .expect("source symlink");

    assert!(!is_package_owned_rustfmt_input(
        &root,
        Path::new("validator/src/lib.rs")
    ));

    std::fs::remove_file(outside).expect("outside cleanup");
    cleanup(root);
}
