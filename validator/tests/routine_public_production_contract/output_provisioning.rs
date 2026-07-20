use super::scenario::{Fixture, pass_node, prefix_route};
use serde_json::Value;
use std::fs;

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
        .expect("ambiguous checkpoint is not JSON");
    assert_eq!(checkpoint["state"], "ambiguous");
    assert_eq!(checkpoint["terminal_outcome"], "ambiguous");
    assert_eq!(checkpoint["operation"], "terminal");

    let recovered = fixture.run();
    assert_eq!(recovered.status.code(), Some(3), "{recovered:?}");
    assert!(recovered.stdout.is_empty(), "{recovered:?}");
    let diagnostic: Value = serde_json::from_slice(&recovered.stderr).unwrap();
    assert_eq!(
        diagnostic["cause"],
        "the routine custody already has a terminal or pending record and no exact continuation was supplied"
    );
    assert_eq!(fs::read(&foreign).unwrap(), b"foreign-user-work");
    fixture.teardown_after_assertions();
}
