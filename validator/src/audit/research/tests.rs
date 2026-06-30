use serde_json::json;

#[test]
fn research_audit_reports_missing_docs_and_registry_row_shape_edges() {
    let root = crate::self_tests::boundaries::support::temp_root("audit-research");
    let missing = super::failures(&root);
    assert!(
        missing
            .iter()
            .any(|failure| failure.starts_with(super::CARDS_PATH)),
        "{missing:#?}"
    );

    std::fs::create_dir_all(root.join("docs")).expect("docs");
    crate::json_boundary::write_json(
        &root.join(super::CARDS_PATH),
        &json!({
            "schema":"wrong",
            "source_cards":[{"source_id":"source-a","requirements":[{"id":"req-a"}]}]
        }),
    )
    .expect("cards");
    crate::json_boundary::write_json(
        &root.join(super::REGISTRY_PATH),
        &json!({"sources":[{
            "source_id":"source-a",
            "source_card_path":"wrong",
            "source_card_digest":"sha256:bad",
            "canonical_law_ids_affected":[],
            "claim_ceiling_impact":""
        }]}),
    )
    .expect("registry");
    crate::json_boundary::write_json(&root.join(super::TRACE_PATH), &json!({"entries":[]}))
        .expect("trace");
    let failures = super::failures(&root);
    assert!(failures.contains(&"research_source_cards_wrong_schema".to_string()));
    assert!(failures.contains(&"research_registry_source_card_path_invalid:source-a".to_string()));
    assert!(failures.contains(&"research_registry_unmapped_source:source-a".to_string()));
    assert!(failures.contains(&"research_registry_missing_claim_ceiling:source-a".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup");
}
