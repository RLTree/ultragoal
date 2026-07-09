use super::{args, minimal_root};
use crate::cli::observe;
use std::fs;

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
fn unknown_observe_subcommand_does_not_route_to_command_roundtrip() {
    let err = observe::parse(&args(&["observe", "fit", "--command", "unknown-command"]))
        .expect_err("unknown observe subcommand fails before command roundtrip authority");
    assert!(err.contains("unknown ultragoal observe command"), "{err}");
}
