use super::fixture::{SourceRoot, snapshot};
use std::path::PathBuf;

#[test]
fn no_write_commands_preserve_recursive_bytes_and_metadata() {
    let root = SourceRoot::new("zero-write");
    let before = snapshot(root.path());
    let line = crate::cli::line_caps::LineCapsCommand {
        receipt: PathBuf::new(),
        jobs: Some(16),
    };
    let namespace = crate::cli::namespace::NamespaceCommand {
        receipt: PathBuf::new(),
        jobs: Some(16),
    };
    let typed = crate::cli::typed_boundaries::TypedBoundariesCommand {
        receipt: PathBuf::new(),
        jobs: Some(16),
    };
    let _ = crate::cli::line_caps::run(root.path(), &line).expect("line caps no-write");
    let _ = crate::cli::namespace::run(root.path(), &namespace).expect("namespace no-write");
    let _ = crate::cli::typed_boundaries::run(root.path(), &typed).expect("typed no-write");
    assert_eq!(snapshot(root.path()), before);
}

#[test]
fn no_write_parser_rejects_receipt_collisions() {
    let arguments = |values: &[&str]| {
        values
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
    };
    let line = crate::cli::line_caps::parse(&arguments(&[
        "line-caps",
        "check",
        "--strict",
        "--no-write",
        "--receipt",
        "out.json",
    ]))
    .expect_err("line collision");
    let namespace = crate::cli::namespace::parse(&arguments(&[
        "namespace",
        "check",
        "--strict",
        "--no-write",
        "--receipt",
        "out.json",
    ]))
    .expect_err("namespace collision");
    let typed = crate::cli::typed_boundaries::parse(&arguments(&[
        "typed-boundaries",
        "check",
        "--strict",
        "--no-write",
        "--receipt",
        "out.json",
    ]))
    .expect_err("typed collision");
    for error in [line, namespace, typed] {
        assert!(error.contains("conflicts with --receipt"), "{error}");
    }
}
