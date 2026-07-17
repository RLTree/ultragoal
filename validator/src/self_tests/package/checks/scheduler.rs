use serde_json::json;

#[test]
fn package_checks_preserve_current_check_context_inside_parallel_text_task() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "package-checks-current-context",
    );
    super::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    super::write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[{"id":"schema-valid"}]}),
    );
    super::write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[{"id":"schema-valid"}]}),
    );
    super::write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[{"obligation_id":"schema-valid"}]}),
    );
    super::write_json(
        &root.join("fixtures/mandatory-law-surfaces/valid/schema-valid.json"),
        &json!({"schema":"harness-ultragoal.mandatory-law-surface-receipt.v1"}),
    );
    super::write_json(
        &root.join("fixtures/red/schema-valid-red.json"),
        &json!({
            "schema": "harness-ultragoal.red-packet.v1",
            "id": "schema-valid-red",
            "expected_failure": {
                "check_id": "schema-valid",
                "error": "mandatory_law_specific_guard_not_enforced:schema-valid:schema_dispatch"
            }
        }),
    );
    super::write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{"id":"schema-valid-red"}]),
    );
    super::write_json(
        &root.join("docs/mandatory-law-surfaces.json"),
        &json!({"laws":[{
            "law_id":"schema-valid",
            "schema":"harness-ultragoal.mandatory-law-surface-receipt.v1",
            "enforcement_status":"deterministic_fail_closed",
            "standards_row_id":"schema-valid",
            "source_obligation_id":"schema-valid",
            "foundational_trace_id":"schema-valid",
            "validator_check_id":"schema-valid",
            "valid_fixture_path":"fixtures/mandatory-law-surfaces/valid/schema-valid.json",
            "claim_ceiling_guard":"blocks",
            "red_fixture_ids":["schema-valid-red"],
            "behavior_failure_modes":["schema-invalid"],
            "law_specific":{"schema_dispatch":true},
            "evidence_artifacts":[]
        }]}),
    );
    let store = crate::schema_catalog::load(&root);
    let results = crate::audit::package::checks::checks_with_scheduler(
        &root,
        &store,
        &[
            "schema-valid".to_string(),
            "source-obligation-coverage".to_string(),
        ],
        &[],
        crate::scheduler::SchedulerConfig::from_jobs(Some(2)).expect("jobs"),
    );
    let source_obligation = results
        .failures
        .get("source-obligation-coverage")
        .cloned()
        .unwrap_or_default();
    assert!(
        !source_obligation
            .iter()
            .any(|failure| failure
                == "mandatory_law_current_check_missing:schema-valid:schema-valid"),
        "{source_obligation:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup current check context");
}
