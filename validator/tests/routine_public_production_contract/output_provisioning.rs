use super::scenario::{Fixture, pass_node, prefix_route};
use serde_json::Value;
use std::fs;

#[test]
fn dependency_closed_outputs_are_journaled_then_reused_by_the_real_children() {
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

    let repeat = fixture.run();
    assert_eq!(repeat.status.code(), Some(0), "{repeat:?}");
    assert!(repeat.stderr.is_empty(), "{repeat:?}");
    let repeat = Fixture::value(&repeat);
    assert_eq!(repeat["status"], "reused");
    assert!(
        repeat["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|node| node["disposition"] == "reused")
    );
    fixture.teardown_after_assertions();
}

#[test]
fn foreign_output_is_preserved_until_exact_pending_recovery_can_continue() {
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
            .contains("\"state\":\"reserved\"")
    );

    fs::remove_file(&foreign).unwrap();
    let recovered = fixture.run();
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    assert!(recovered.stderr.is_empty(), "{recovered:?}");
    assert_eq!(Fixture::value(&recovered)["status"], "executed");
    assert_eq!(fs::read_dir(&scope).unwrap().count(), 0);
    fixture.teardown_after_assertions();
}
