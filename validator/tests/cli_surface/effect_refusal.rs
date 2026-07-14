use super::scenario::*;

#[cfg(unix)]
#[test]
fn unavailable_effectful_routes_refuse_before_repository_mutation_or_path_echo() {
    let repository = Repository::new("effect-refusal");
    let initial = observe(&repository.root);
    for args in [
        &[
            "--json",
            "package",
            "build",
            "--output",
            "private-output.json",
        ][..],
        &[
            "--json",
            "prove",
            "--claim",
            "private-claim",
            "--output",
            "private-proof.json",
        ][..],
        &[
            "--json",
            "migrate",
            "apply",
            "--plan",
            "private-plan.json",
            "--accept-plan",
            "private-plan-id",
        ][..],
    ] {
        let before = observe(&repository.root);
        let output = repository.run(args);
        assert_eq!(output.status.code(), Some(4), "{args:?}: {output:?}");
        let value: serde_json::Value =
            serde_json::from_slice(&emitted(&output)).expect("authority refusal JSON");
        assert_eq!(value["exit_class"], "unsupported_capability");
        let bytes = emitted(&output);
        let text = String::from_utf8_lossy(&bytes);
        for private in [
            "private-output.json",
            "private-claim",
            "private-proof.json",
            "private-plan.json",
            "private-plan-id",
        ] {
            assert!(!text.contains(private), "private argument echo: {private}");
        }
        assert_eq!(observe(&repository.root), before, "hidden write: {args:?}");
    }
    assert_eq!(observe(&repository.root), initial);
}
