use super::scenario::{Fixture, pass_node, prefix_route};
use std::fs;

#[test]
fn retired_probe_ignore_rule_is_compatibility_only_and_host_event_remains_owner_only() {
    let mut fixture = Fixture::new(
        "probe-only-observability-ignore",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::write(
        fixture.root.join(".gitignore"),
        b"target/\nvalidation_artifacts/observability/spool/successor-events-probe.jsonl\n",
    )
    .unwrap();
    let before_status = fixture.status();

    let executed = fixture.run();

    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    assert_eq!(Fixture::value(&executed)["status"], "executed");
    assert_eq!(fixture.status(), before_status);
    let _host_event = fixture.event_leaf();
    assert!(
        !fixture
            .root
            .join("validation_artifacts/observability/spool")
            .exists()
    );
    fixture.teardown_after_assertions();
}
