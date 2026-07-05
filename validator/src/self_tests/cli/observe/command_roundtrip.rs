use super::{args, minimal_root};
use crate::cli::observe;
use crate::cli::observe::types::ObserveOperation;
use std::fs;
use std::path::PathBuf;

#[test]
fn observe_run_routes_command_roundtrip_without_substituting_unknown_specs() {
    let root = minimal_root("observe-command-roundtrip-route");
    let command = observe::parse(&args(&[
        "observe",
        "command-roundtrip",
        "--command",
        "unknown-command",
    ]))
    .expect("parse")
    .expect("observe command roundtrip");
    let err =
        observe::run(&root, &command).expect_err("unknown command roundtrip spec fails closed");
    assert!(err.contains("unknown observability command spec"), "{err}");
    fs::remove_dir_all(root).expect("cleanup observe command roundtrip route");
}

#[test]
fn public_alias_routes_to_command_roundtrip_runner() {
    let root = minimal_root("observe-command-roundtrip-public-route");
    let command = observe::parse(&args(&["observe", "fit", "--command", "unknown-command"]))
        .expect("parse")
        .expect("observe public alias command");
    assert_eq!(command.operation, ObserveOperation::CommandRoundtrip);
    assert_eq!(
        command.operation.receipt_rel(),
        PathBuf::from("validation_artifacts/observability/observe-command-roundtrip.json")
    );
    let err = observe::run(&root, &command).expect_err("unknown target fails closed");
    assert!(err.contains("unknown observability command spec"), "{err}");
    fs::remove_dir_all(root).expect("cleanup observe command roundtrip public route");
}
