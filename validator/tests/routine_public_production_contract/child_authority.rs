#[path = "child_authority_support.rs"]
mod support;

use super::scenario::tree;
use support::*;

#[test]
fn direct_public_binary_cannot_select_child_behavior_from_environment() {
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
fn copied_or_replayed_child_environment_has_no_bearer_authority() {
    let fixture = dirty_fixture("copied-replayed-environment");
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    for _ in 0..2 {
        let mut command = fixture.base_command();
        command
            .args(["--json", "check", "routine"])
            .env("HUL_ROUTINE_CHILD_FD", CAPABILITY_FD.to_string());
        assert_refused(&run_with_frame(command, frame(&fixture)));
    }
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
}

#[test]
fn wrong_parent_or_process_session_cannot_authorize_framed_input() {
    let fixture = dirty_fixture("wrong-parent-session");
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    assert_refused(&run_with_forged_channel(&fixture, false, None).0);
    assert_refused(&run_with_forged_channel(&fixture, true, None).0);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
}

#[test]
fn attacker_sealed_material_still_cannot_cross_process_binding() {
    let fixture = dirty_fixture("attacker-sealed-cross-process");
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let (first, material) = run_with_forged_channel(&fixture, false, None);
    assert_refused(&first);
    let (later, _) = run_with_forged_channel(&fixture, false, Some(&material));
    assert_refused(&later);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
}
