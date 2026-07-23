use super::scenario::{Fixture, pass_node, prefix_route};
use serde_json::Value;
use std::fs;

#[test]
fn ignoring_only_the_retired_probe_refuses_before_effect_or_journal_creation() {
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

    let refused = fixture.run();

    assert_eq!(refused.status.code(), Some(4), "{refused:?}");
    assert!(refused.stdout.is_empty(), "{refused:?}");
    let value: Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(
        value["cause"],
        "routine-runtime-observability-store-not-ignored"
    );
    assert_eq!(fixture.status(), before_status);
    assert!(!fixture.root.join("target/routine").exists());
    assert!(!fixture.checkpoint_path().exists());
    assert!(
        !fixture
            .root
            .join("validation_artifacts/observability/spool")
            .exists()
    );
    fixture.teardown_after_assertions();
}
