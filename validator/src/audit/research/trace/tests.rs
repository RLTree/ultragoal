use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn trace_row_failures_cover_unknowns_missing_paths_and_fields() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("audit-research-trace");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::create_dir_all(root.join("templates/agent-standards")).expect("standards");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["docs/present-schema.json", "schemas/observability-event.schema.json"]}),
    )
    .expect("manifest");
    std::fs::write(root.join("docs/present-schema.json"), "{}").expect("schema");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas");
    std::fs::write(root.join("schemas/observability-event.schema.json"), "{}")
        .expect("observability schema");
    crate::json_boundary::write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[
            {"id":"known-standard"},
            {"id":"full-local-observability-stack-integration-non-opaque-failure"}
        ]}),
    )
    .expect("standards");
    crate::json_boundary::write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[
            {"id":"known-obligation"},
            {"id":"full-local-observability-stack-integration-non-opaque-failure"}
        ]}),
    )
    .expect("obligations");
    crate::json_boundary::write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[
            {"obligation_id":"known-trace"},
            {"obligation_id":"full-local-observability-stack-integration-non-opaque-failure"}
        ]}),
    )
    .expect("foundational");
    crate::json_boundary::write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{"id":"known-red"}]),
    )
    .expect("red");

    let requirements = BTreeMap::from([
        ("req-a".to_string(), "source-a".to_string()),
        ("req-good".to_string(), "source-good".to_string()),
        ("req-missing".to_string(), "source-missing".to_string()),
    ]);
    let row = json!({
        "source_id":"other-source",
        "standards_row_ids":["missing-standard"],
        "source_obligation_ids":["missing-obligation"],
        "foundational_trace_ids":["missing-trace"],
        "validator_check_ids":["missing-check"],
        "red_fixture_ids":["missing-red"],
        "tamper_fixture_ids":["missing-tamper"],
        "green_fixture_paths":["docs/missing-green.json"],
        "schemas":["docs/missing-schema.json"],
        "package_inventory_paths":["docs/not-in-package.json"],
        "setup_retrofit_outputs":["docs/not-in-package.json"],
        "canonical_law_ids":["HU-001"],
        "receipt_requirements":[],
        "claim_guards":[],
        "final_packet_fields":[],
        "update_goal_blockers":[]
    });
    let good_row = json!({
        "source_id":"source-good",
        "standards_row_ids":["known-standard", "full-local-observability-stack-integration-non-opaque-failure"],
        "source_obligation_ids":["known-obligation", "full-local-observability-stack-integration-non-opaque-failure"],
        "foundational_trace_ids":["known-trace", "full-local-observability-stack-integration-non-opaque-failure"],
        "validator_check_ids":["agent-standards-enforcement", "full-local-observability-stack-integration-non-opaque-failure"],
        "red_fixture_ids":["known-red"],
        "tamper_fixture_ids":["known-red"],
        "green_fixture_paths":["docs/present-schema.json"],
        "schemas":["schemas/observability-event.schema.json"],
        "package_inventory_paths":["schemas/observability-event.schema.json"],
        "setup_retrofit_outputs":["docs/present-schema.json"],
        "canonical_law_ids":["research-source-law", "full-local-observability-stack-integration-non-opaque-failure"],
        "receipt_requirements":["observability receipt"],
        "claim_guards":["observability_claims_withheld_without_typed_queryable_correlated_receipt"],
        "final_packet_fields":["observability_status"],
        "update_goal_blockers":["observability_gate_incomplete"]
    });
    let trace = BTreeMap::from([
        ("req-a".to_string(), row.clone()),
        ("req-good".to_string(), good_row),
        ("unknown-req".to_string(), row),
    ]);
    let coverage = super::coverage_failures(&requirements, &trace);
    assert!(coverage.contains(&"research_trace_missing_requirement:req-missing".to_string()));
    let failures = super::row_failures(&root, &requirements, &trace);
    assert!(failures.contains(&"research_trace_source_mismatch:req-a:source-a".to_string()));
    assert!(failures.contains(&"research_trace_unknown_requirement:unknown-req".to_string()));
    assert!(
        failures.contains(&"research_trace_unknown_standard:req-a:missing-standard".to_string())
    );
    assert!(failures.contains(&"research_trace_unknown_check:req-a:missing-check".to_string()));
    assert!(
        failures
            .contains(&"research_trace_missing_schema:req-a:docs/missing-schema.json".to_string())
    );
    assert!(
        failures
            .contains(&"research_trace_unknown_tamper_fixture:req-a:missing-tamper".to_string())
    );
    assert!(failures.contains(
        &"research_trace_setup_retrofit_omitted:req-a:docs/not-in-package.json".to_string()
    ));
    assert!(failures.contains(&"research_trace_alias_only:req-a".to_string()));
    assert!(failures.contains(&"research_trace_missing_receipt_requirements:req-a".to_string()));
    assert!(failures.contains(&"research_trace_missing_claim_guards:req-a".to_string()));
    assert!(failures.contains(&"research_trace_missing_final_packet_fields:req-a".to_string()));
    assert!(failures.contains(&"research_trace_missing_update_goal_blockers:req-a".to_string()));
    assert!(!failures.iter().any(|failure| failure.contains("req-good")));
    std::fs::remove_dir_all(root).expect("cleanup");
}
