use super::scenario::{Fixture, pass_node, prefix_route, routine_command_from_current_directory};
use serde_json::Value;
use std::fs;

#[test]
fn current_directory_is_the_supported_default_routine_root() {
    let mut fixture = Fixture::new(
        "current-directory-root",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        false,
        false,
    );
    let output =
        routine_command_from_current_directory(&fixture.root, &fixture.home, fixture.binary_path())
            .args(["--json", "check", "routine"])
            .output()
            .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(Fixture::value(&output)["status"], "clean-no-op");
    fixture.teardown_after_assertions();
}

#[test]
fn dirty_routine_preserves_ignored_build_hardlinks() {
    let mut fixture = Fixture::new(
        "ignored-build-hardlink",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let ignored = fixture.root.join("target/prior-build/object");
    fs::create_dir_all(ignored.parent().unwrap()).unwrap();
    fs::write(&ignored, b"reproducible build output\n").unwrap();
    fs::hard_link(
        &ignored,
        fixture.root.join("target/prior-build/object-copy"),
    )
    .unwrap();

    let output =
        routine_command_from_current_directory(&fixture.root, &fixture.home, fixture.binary_path())
            .args(["--json", "check", "routine"])
            .output()
            .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(Fixture::value(&output)["status"], "executed");
    assert_eq!(fs::read(&ignored).unwrap(), b"reproducible build output\n");
    fixture.teardown_after_assertions();
}

#[test]
fn dependency_closed_outputs_are_journaled_before_real_children_then_repeat_reuses() {
    let mut fixture = Fixture::new(
        "multi-node-output-journal",
        &[pass_node("syntax", &[]), pass_node("compile", &["syntax"])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let first = fixture.run();
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    assert!(first.stderr.is_empty(), "{first:?}");
    let first = Fixture::value(&first);
    assert_eq!(first["status"], "executed");
    assert_eq!(first["nodes"].as_array().unwrap().len(), 2);
    assert!(fixture.root.join("target/routine/syntax").is_dir());
    assert!(fixture.root.join("target/routine/compile").is_dir());
    assert!(
        String::from_utf8(
            fs::read(fixture.authority_root().join("routine-authority.state")).unwrap(),
        )
        .unwrap()
        .contains("\"state\":\"complete\"")
    );

    let before = super::scenario::tree(&fixture.root);
    let repeat = fixture.run();
    assert_eq!(repeat.status.code(), Some(0), "{repeat:?}");
    assert!(repeat.stderr.is_empty(), "{repeat:?}");
    assert_eq!(Fixture::value(&repeat)["status"], "reused");
    assert_eq!(Fixture::value(&repeat)["effect"], "none");
    assert_eq!(super::scenario::tree(&fixture.root), before);
    fixture.teardown_after_assertions();
}

#[test]
fn foreign_output_is_preserved_and_fresh_process_recovery_refuses() {
    let mut fixture = Fixture::new(
        "foreign-output-recovery",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let scope = fixture.root.join("target/routine/compile");
    let foreign = scope.join("foreign");
    fs::create_dir_all(&scope).unwrap();
    fs::write(&foreign, b"foreign-user-work").unwrap();

    let refusal = fixture.run();
    assert_eq!(refusal.status.code(), Some(3), "{refusal:?}");
    assert!(refusal.stdout.is_empty(), "{refusal:?}");
    let diagnostic: Value = serde_json::from_slice(&refusal.stderr).unwrap();
    assert_eq!(diagnostic["cause"], "mediator-output-scope-not-empty");
    assert_eq!(
        diagnostic["effect"],
        "none_or_unacknowledged_workspace_request"
    );
    assert_eq!(fs::read(&foreign).unwrap(), b"foreign-user-work");
    let durable = fs::read(fixture.authority_root().join("routine-authority.state")).unwrap();
    assert!(
        String::from_utf8(durable)
            .unwrap()
            .contains("\"state\":\"failed\"")
    );
    let checkpoint: Value = serde_json::from_slice(&fs::read(fixture.checkpoint_path()).unwrap())
        .expect("terminal checkpoint is not JSON");
    assert_eq!(checkpoint["state"], "terminal-event-joined");
    assert_eq!(checkpoint["terminal_outcome"], "failed");
    assert_eq!(checkpoint["operation"], "terminal");
    let event_leaf = fixture.event_leaf();
    let event_before_recovery = fs::read(&event_leaf).unwrap();
    let event: Value = serde_json::from_slice(&event_before_recovery).unwrap();
    assert_eq!(event["event"]["outcome"], "fail");
    assert_eq!(
        event["event"]["public_attributes"]["routine_terminal_outcome"],
        "failed"
    );

    let before_recovery_root = super::scenario::tree(&fixture.root);
    let before_recovery_home = super::scenario::tree(&fixture.home);
    let before_recovery_status = fixture.status();
    let recovered = fixture.run();
    assert_eq!(recovered.status.code(), Some(3), "{recovered:?}");
    assert!(recovered.stdout.is_empty(), "{recovered:?}");
    let diagnostic: Value = serde_json::from_slice(&recovered.stderr).unwrap();
    assert_eq!(
        diagnostic["cause"],
        "the routine custody already has a terminal or pending record and no exact continuation was supplied"
    );
    assert_eq!(fs::read(&foreign).unwrap(), b"foreign-user-work");
    assert_eq!(fs::read(&event_leaf).unwrap(), event_before_recovery);
    assert_eq!(super::scenario::tree(&fixture.root), before_recovery_root);
    assert_eq!(super::scenario::tree(&fixture.home), before_recovery_home);
    assert_eq!(fixture.status(), before_recovery_status);
    fixture.teardown_after_assertions();
}

#[test]
fn unsettled_reserved_attempt_is_not_projected_as_a_terminal_event() {
    let mut fixture = Fixture::new(
        "unsettled-reserved-no-terminal-projection",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let interrupted = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    let checkpoint: Value =
        serde_json::from_slice(&fs::read(fixture.checkpoint_path()).unwrap()).unwrap();
    assert_eq!(checkpoint["state"], "reserved");
    assert_eq!(checkpoint["terminal_outcome"], Value::Null);
    assert_eq!(host_event_leaf_count(&fixture), 0);

    let before_root = super::scenario::tree(&fixture.root);
    let before_home = super::scenario::tree(&fixture.home);
    let before_status = fixture.status();
    let refused = fixture.run();
    assert_eq!(refused.status.code(), Some(3), "{refused:?}");
    assert!(refused.stdout.is_empty(), "{refused:?}");
    assert_eq!(host_event_leaf_count(&fixture), 0);
    assert_eq!(super::scenario::tree(&fixture.root), before_root);
    assert_eq!(super::scenario::tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    fixture.teardown_after_assertions();
}

fn host_event_leaf_count(fixture: &Fixture) -> usize {
    fs::read_dir(fixture.state_root().join("adapter"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("routine-events-") && name.ends_with(".jsonl"))
        })
        .count()
}
