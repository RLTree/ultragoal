use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn foundational_law_surface_inventory_requires_cross_surface_edges() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("law-surface-inventory");
    std::fs::create_dir_all(&root).expect("temp root");
    let mandatory = json!({
        "laws":[{
            "law_id":"typed-records-over-prose",
            "validator_check_id":"typed-records-over-prose",
            "claim_ceiling_guard":"typed_records_claims_withheld",
            "source_obligation_id":"missing-obligation",
            "foundational_trace_id":"missing-trace",
            "standards_row_id":"missing-standards",
            "valid_fixture_path":"docs/ultragoal-contract-2026-07/README.md",
            "red_fixture_ids":["missing-red"],
            "evidence_artifacts":[{
                "path":"docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md"
            }]
        }]
    });
    let failures = crate::audit::law::authority_surfaces::authority_graph_failures_for_test(
        &root,
        &BTreeSet::new(),
        &mandatory,
        &json!({"obligations":[]}),
        &json!({"entries":[]}),
        &json!({"rows":[]}),
        "",
        &BTreeSet::new(),
    );
    assert!(
        failures.iter().any(|(_, failure)| failure
            .contains("foundational_law_surface_missing_source_obligation_row")),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|(_, failure)| failure
            .contains("foundational_law_surface_missing_foundational_trace_row")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("foundational_law_surface_missing_standards_row")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("builder_contract_path_used_as_law_surface")),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|(_, failure)| failure
            .contains("foundational_law_surface_red_fixture_missing_from_catalog")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup law surface inventory");
}

#[test]
fn foundational_law_surface_inventory_accepts_complete_product_edges() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("law-surface-complete");
    let paths = [
        "fixtures/mandatory-law-surfaces/valid/typed-records-over-prose.json",
        "fixtures/red/typed-records-over-prose-raw-downstream-json-authority-rejected-red.json",
        "docs/source-obligation-matrix.json",
        "validator/src/audit/law/authority_surfaces/source/raw/mod.rs",
    ];
    for rel in paths {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        std::fs::write(path, "{}").expect("surface file");
    }
    let inventory = paths.into_iter().map(str::to_string).collect();
    let mandatory = json!({
        "laws":[{
            "law_id":"typed-records-over-prose",
            "validator_check_id":"typed-records-over-prose",
            "claim_ceiling_guard":"typed_records_claims_withheld",
            "source_obligation_id":"typed-records-over-prose",
            "foundational_trace_id":"typed-records-over-prose",
            "standards_row_id":"typed-records-over-prose",
            "valid_fixture_path":"fixtures/mandatory-law-surfaces/valid/typed-records-over-prose.json",
            "red_fixture_ids":["typed-records-over-prose-raw-downstream-json-authority-rejected-red"],
            "evidence_artifacts":[{
                "path":"validator/src/audit/law/authority_surfaces/source/raw/mod.rs"
            }]
        }]
    });
    let trace = json!({
        "entries":[{
            "law_id":"typed-records-over-prose",
            "obligation_id":"typed-records-over-prose",
            "source_artifact":{"path":"docs/source-obligation-matrix.json"},
            "valid_fixture_id":"fixtures/mandatory-law-surfaces/valid/typed-records-over-prose.json",
            "red_fixture_id":"typed-records-over-prose-raw-downstream-json-authority-rejected-red",
            "validator_check_id":"typed-records-over-prose",
            "claim_ceiling_impact":"typed_records_claims_withheld"
        }]
    });
    let red_ids = std::iter::once(
        "typed-records-over-prose-raw-downstream-json-authority-rejected-red".to_string(),
    )
    .collect();
    let failures = crate::audit::law::authority_surfaces::authority_graph_failures_for_test(
        &root,
        &inventory,
        &mandatory,
        &json!({"obligations":[{"id":"typed-records-over-prose"}]}),
        &trace,
        &json!({"rows":[{"id":"typed-records-over-prose"}]}),
        "typed-records-over-prose\tpass\tvalidator/src/audit/law/authority_surfaces/source/raw/mod.rs\tsha256:test\t2026-07-02T00:00:00Z\tcompletion\n",
        &red_ids,
    );
    assert!(failures.is_empty(), "{failures:?}");
    std::fs::remove_dir_all(root).expect("cleanup law surface complete");
}

#[test]
fn foundational_law_surface_inventory_rejects_invalid_paths_and_missing_red_edges() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("law-surface-invalid-path");
    std::fs::create_dir_all(&root).expect("invalid path fixture root");
    let private_artifact_path = std::path::PathBuf::from(std::path::MAIN_SEPARATOR.to_string())
        .join("private")
        .join("tmp")
        .join("receipt.json")
        .to_string_lossy()
        .into_owned();
    let mandatory = json!({
        "laws":[{
            "law_id":"authority-source-binding",
            "validator_check_id":"authority-source-binding",
            "claim_ceiling_guard":"authority_claims_withheld",
            "source_obligation_id":"authority-source-binding",
            "foundational_trace_id":"authority-source-binding",
            "standards_row_id":"authority-source-binding",
            "valid_fixture_path":"../outside.json",
            "evidence_artifacts":[{"path":private_artifact_path}]
        }]
    });
    let failures = crate::audit::law::authority_surfaces::authority_graph_failures_for_test(
        &root,
        &BTreeSet::new(),
        &mandatory,
        &json!({"obligations":[{"id":"authority-source-binding"}]}),
        &json!({}),
        &json!({"rows":[{"id":"authority-source-binding"}]}),
        "authority-source-binding\tpass\tvalidator/src/audit/law/authority_surfaces/graph_edges.rs\tsha256:test\t2026-07-02T00:00:00Z\tcompletion\n",
        &BTreeSet::new(),
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("foundational_law_surface_path_invalid")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|(_, failure)| failure.contains("field=red_fixture_ids")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup law surface invalid path");
}
