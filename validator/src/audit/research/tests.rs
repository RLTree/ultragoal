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
            "sources":[
                {
                    "source_id":"source-a",
                    "requirements":[{
                        "requirement_id":"req-a",
                        "summary":"summary-only source card without anchors"
                    }]
                },
                {
                    "source_id":"source-b",
                    "canonical_url":"https://example.test/source-b",
                    "source_kind":"public_web",
                    "source_artifact_digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "source_artifact_method":"http_body_sha256",
                    "requirements":[{
                        "requirement_id":"req-b",
                        "summary":"",
                        "source_evidence_ids":["anchor-b"]
                    }],
                    "evidence_anchors":[{
                        "evidence_id":"anchor-b",
                        "source_locator":"https://wrong.example/source-b#anchor-b",
                        "source_signal":"",
                        "requirement_ids":["unknown-req"]
                    }]
                }
            ]
        }),
    )
    .expect("cards");
    crate::json_boundary::write_json(
        &root.join(super::REGISTRY_PATH),
        &json!({"sources":[{
            "source_id":"source-a",
            "source_card_path":"wrong",
            "source_card_digest":"sha256:bad",
            "source_artifact_digest":"sha256:bad",
            "source_artifact_method":"wrong",
            "canonical_law_ids_affected":[],
            "claim_ceiling_impact":""
        }]}),
    )
    .expect("registry");
    crate::json_boundary::write_json(&root.join(super::TRACE_PATH), &json!({"entries":[]}))
        .expect("trace");
    let failures = super::failures(&root);
    assert!(failures.contains(&"research_source_cards_wrong_schema".to_string()));
    assert!(failures.contains(&"research_source_card_canonical_url_missing:source-a".to_string()));
    assert!(failures.contains(&"research_source_card_kind_missing:source-a".to_string()));
    assert!(
        failures.contains(&"research_source_card_artifact_digest_missing:source-a".to_string())
    );
    assert!(
        failures.contains(&"research_source_card_artifact_method_missing:source-a".to_string())
    );
    assert!(failures.contains(&"research_source_card_evidence_missing:source-a".to_string()));
    assert!(failures.contains(&"research_source_card_requirement_unanchored:req-a".to_string()));
    assert!(
        failures.contains(&"research_source_card_requirement_summary_missing:req-b".to_string())
    );
    assert!(
        failures.contains(
            &"research_source_card_evidence_locator_mismatch:source-b:anchor-b".to_string()
        )
    );
    assert!(
        failures.contains(
            &"research_source_card_evidence_signal_missing:source-b:anchor-b".to_string()
        )
    );
    assert!(
        failures.contains(
            &"research_source_card_evidence_unknown_requirement:source-b:anchor-b:unknown-req"
                .to_string()
        )
    );
    assert!(failures.contains(&"research_registry_source_card_path_invalid:source-a".to_string()));
    assert!(
        failures.contains(&"research_registry_source_artifact_digest_stale:source-a".to_string())
    );
    assert!(
        failures.contains(&"research_registry_source_artifact_method_stale:source-a".to_string())
    );
    assert!(failures.contains(&"research_registry_unmapped_source:source-a".to_string()));
    assert!(failures.contains(&"research_registry_missing_claim_ceiling:source-a".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup");
}
