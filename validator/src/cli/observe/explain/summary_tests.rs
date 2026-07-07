use super::*;

#[test]
fn root_cause_and_paths_cover_defensive_empty_failure_boundaries() {
    let known = json!([]);
    let ctx = ExplainContext {
        candidate: "sha256:current",
        target_requested: None,
        observed: None,
        stale_observed: None,
        missing_observed: None,
        opaque_observed: None,
        known_current_failure: &known,
        repair_guidance: "repair specific blocker",
    };

    assert_eq!(root_cause(&ctx), "no current failure identified");
    assert_eq!(
        narrow_rerun(None),
        "run the requested target command once, then rerun explain"
    );
}

#[test]
fn implicated_paths_include_artifact_and_receipt_when_present() {
    let paths = implicated_paths(Some(&json!({
        "artifact_path": "validation_artifacts/coverage",
        "receipt_path": "validation_artifacts/coverage/coverage-receipt.json"
    })));

    assert_eq!(
        paths,
        json!([
            "validation_artifacts/coverage",
            "validation_artifacts/coverage/coverage-receipt.json"
        ])
    );
}

#[test]
fn narrow_rerun_for_observe_query_includes_target_selector() {
    let rerun = narrow_rerun(Some(&json!({
        "operation": "observe.traces.query",
        "run_id": "run-target",
        "correlation_id": "corr-target",
        "command": "ultragoal",
        "subcommand": "observe traces"
    })));

    assert_eq!(
        rerun,
        "ultragoal observe traces query --run-id run-target --correlation-id corr-target --limit 100"
    );
}
