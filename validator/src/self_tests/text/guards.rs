use serde_json::json;

#[test]
fn text_guards_reject_source_card_private_path_and_moving_value_overclaims() {
    let source_cards = json!([
        {
            "id":"stale-current",
            "retrieval_receipt":{"status":"not_refreshed"},
            "notes":"Current implementation source-backed claim",
            "cited_claims":[]
        },
        {
            "id":"stale-lowered",
            "retrieval_receipt":{"status":"not_refreshed"},
            "notes":"Current implementation claim ceiling says verify exact",
            "cited_claims":[]
        },
        {
            "id":"fresh-current",
            "retrieval_receipt":{"status":"refreshed"},
            "notes":"current implementation",
            "cited_claims":[]
        }
    ]);
    let source_failures = crate::audit::text_guards::source_card_value_failures(&source_cards);
    assert_eq!(
        source_failures,
        vec!["stale-current: source_card_not_refreshed_overclaim"]
    );

    assert_eq!(
        crate::audit::text_guards::private_path_value_failures(&json!({
            "path":format!("{}/tester/private-proof.json", private_home_prefix())
        })),
        vec!["manifest_owned_private_local_path"]
    );
    assert_eq!(
        crate::audit::text_guards::private_path_value_failures(&json!({
            "path":format!("{}/proof.json", private_tmp_prefix())
        })),
        vec!["manifest_owned_private_local_path"]
    );
    assert!(
        crate::audit::text_guards::private_path_value_failures(&json!({"path":"docs/proof.json"}))
            .is_empty()
    );

    let moving = crate::audit::text_guards::moving_value_value_failures(&json!({
        "sha":"Digest sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "count":"140/143 checks passed",
        "run":"ultragoal-audit-2026-06-26T00:00:00Z",
        "ignored":[
            "validation_artifacts/coverage has sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "historical 1/2 checks",
            "example sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        ]
    }));
    assert_eq!(moving.len(), 3, "{moving:?}");
    assert!(
        moving
            .iter()
            .all(|row| row.contains("moving_value_drift_in_stable_text"))
    );
}

#[test]
fn text_guards_reject_stale_review_phrases_and_manifest_owned_private_paths() {
    let stale = crate::audit::text_guards::stale_review_law_value_failures(&json!({
        "review":"GPT-5.4-mini gave provisional approval from all six reviewers"
    }));
    assert!(
        stale
            .iter()
            .any(|row| row.contains("stale_review_cadence_in_current_surface: gpt-5.4-mini")),
        "{stale:?}"
    );
    assert!(
        stale.iter().any(
            |row| row.contains("stale_review_cadence_in_current_surface: provisional approval")
        ),
        "{stale:?}"
    );
    assert!(
        stale
            .iter()
            .any(|row| row.contains("stale_review_cadence_in_current_surface: all six")),
        "{stale:?}"
    );

    let root = crate::self_tests::boundaries::support::temp_root("text-guard-private-paths");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::create_dir_all(root.join("fixtures/red")).expect("red fixtures");
    std::fs::write(
        root.join("docs/private.md"),
        format!("{}/tester/proof.json", private_home_prefix()),
    )
    .expect("private doc");
    std::fs::write(
        root.join("fixtures/red/private-example.md"),
        format!("{}/example/proof.json", private_home_prefix()),
    )
    .expect("private red example");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({
            "resources":["docs/private.md","fixtures/red/private-example.md"]
        }))
        .expect("manifest bytes"),
    )
    .expect("manifest");
    let private_failures = crate::audit::text_guards::private_home_path_failures(&root);
    assert_eq!(
        private_failures,
        vec!["docs/private.md: manifest_owned_private_local_path"]
    );
    std::fs::remove_dir_all(root).expect("cleanup text guard private paths");
}

fn private_home_prefix() -> &'static str {
    concat!("/", "Users")
}

fn private_tmp_prefix() -> &'static str {
    concat!("/", "private", "/tmp")
}
