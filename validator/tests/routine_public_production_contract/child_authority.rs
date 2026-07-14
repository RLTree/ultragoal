#[path = "child_authority_support.rs"]
mod support;

use super::scenario::tree;
use support::*;

#[test]
fn direct_public_binary_cannot_select_child_behavior_from_legacy_environment() {
    let fixture = dirty_fixture("direct-forged-environment");
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let mut command = fixture.base_command();
    command
        .args(["--json", "check", "routine"])
        .env("HUL_ROUTINE_BEHAVIOR_ID", "rust-source-syntax-v1")
        .env("HUL_ROUTINE_REQUEST_ID", "attacker-request")
        .env("HUL_ROUTINE_PROTOCOL_ID", "attacker-protocol")
        .env("HUL_ROUTINE_INTENT_ID", "attacker-intent")
        .env("HUL_ROUTINE_NODE_ID", "attacker-node");
    let output = run_with_frame(command, frame(&fixture));
    assert_refused(&output);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
}

#[test]
fn copied_or_replayed_descriptor_selector_is_refusal_only() {
    let fixture = dirty_fixture("copied-replayed-descriptor");
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    for _ in 0..2 {
        let mut command = fixture.base_command();
        command
            .args(["--json", "check", "routine"])
            .env("HUL_ROUTINE_CHILD_FD", CAPABILITY_FD.to_string());
        assert_child_refused(&run_with_frame(command, frame(&fixture)));
    }
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
}

#[test]
fn prebuffered_socket_and_arbitrary_canonical_frame_refuse_without_writes() {
    let fixture = dirty_fixture("prebuffered-socket");
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let output = run_with_prebuffered_channel(&fixture, &legacy_capability_wire());
    assert_child_refused(&output);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
}

#[test]
fn valid_socket_and_held_open_stdin_are_not_read_before_refusal() {
    let fixture = dirty_fixture("blocking-socket-stdin");
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let output = run_with_blocking_socket_and_stdin(&fixture);
    assert_child_refused(&output);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
}

#[test]
fn replayed_prebuffered_socket_material_never_becomes_authority() {
    let fixture = dirty_fixture("prebuffered-replay");
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let wire = legacy_capability_wire();
    assert_child_refused(&run_with_prebuffered_channel(&fixture, &wire));
    assert_child_refused(&run_with_prebuffered_channel(&fixture, &wire));
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
}
