use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn research_audit_reports_missing_docs_and_registry_row_shape_edges() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("audit-research");
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
                        "source_corpus_path":"artifacts/source-snapshots/source-b.txt",
                        "source_corpus_digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
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
            "source_corpus_path":"wrong",
            "source_corpus_digest":"sha256:bad",
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
    assert!(failures.contains(&"research_source_card_corpus_path_missing:source-a".to_string()));
    assert!(failures.contains(&"research_source_card_corpus_digest_missing:source-a".to_string()));
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
    assert!(failures.contains(&"research_registry_source_corpus_path_stale:source-a".to_string()));
    assert!(
        failures.contains(&"research_registry_source_corpus_digest_stale:source-a".to_string())
    );
    assert!(failures.contains(&"research_source_corpus_path_missing:source-a".to_string()));
    assert!(failures.contains(&"research_registry_unmapped_source:source-a".to_string()));
    assert!(failures.contains(&"research_registry_missing_claim_ceiling:source-a".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn research_source_cards_reject_missing_anchor_id_locator_and_requirement_edges() {
    let cards = json!({
        "sources":[{
            "canonical_url":"https://example.test/source",
            "source_kind":"public_web",
            "source_artifact_digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "source_artifact_method":"http_body_sha256",
            "source_corpus_path":"artifacts/source-snapshots/source.txt",
            "source_corpus_digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "requirements":[{
                "summary":"missing requirement id",
                "source_evidence_ids":[""]
            }],
            "evidence_anchors":[{
                "source_signal":"article section",
                "requirement_ids":[]
            }]
        }]
    });

    let failures = super::catalog::source_evidence_failures(&cards);

    assert!(failures.contains(&"research_source_card_id_missing".to_string()));
    assert!(failures.contains(&"research_source_card_requirement_id_missing:".to_string()));
    assert!(failures.contains(&"research_source_card_evidence_id_missing:".to_string()));
    assert!(failures.contains(&"research_source_card_evidence_locator_missing::".to_string()));
    assert!(failures.contains(&"research_source_card_evidence_requirement_missing::".to_string()));
}

#[test]
fn research_source_corpus_guards_reject_escape_stale_digest_and_nonfile_digest_errors() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("audit-research-corpus");
    let rel = "artifacts/source-snapshots/source.txt";
    std::fs::create_dir_all(root.join("artifacts/source-snapshots")).expect("corpus dir");
    std::fs::write(root.join(rel), b"source corpus").expect("corpus");
    let mut package_paths = BTreeSet::new();
    package_paths.insert(rel.to_string());

    let stale = super::source_corpus_failures(
        &root,
        "source-a",
        rel,
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        &package_paths,
    );
    assert!(stale.contains(&format!(
        "research_source_corpus_digest_stale:source-a:{rel}"
    )));

    let invalid = super::source_corpus_failures(
        &root,
        "source-a",
        "../outside.txt",
        "sha256:unused",
        &package_paths,
    );
    assert!(
        invalid
            .iter()
            .any(|failure| failure.starts_with("research_source_corpus_path_invalid:source-a:")),
        "{invalid:#?}"
    );

    let dir_rel = "artifacts/source-snapshots/not-a-file";
    std::fs::create_dir_all(root.join(dir_rel)).expect("non-file corpus");
    package_paths.insert(dir_rel.to_string());
    let digest_error =
        super::source_corpus_failures(&root, "source-a", dir_rel, "sha256:unused", &package_paths);
    assert!(
        digest_error
            .iter()
            .any(|failure| failure.starts_with("research_source_corpus_digest_error:source-a:")),
        "{digest_error:#?}"
    );

    std::fs::remove_dir_all(root).expect("cleanup");
}
