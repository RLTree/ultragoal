use serde_json::json;

#[test]
fn text_guards_reject_source_card_overclaims() {
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
}

#[test]
fn text_guards_reject_manifest_owned_private_paths() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("text-guard-private-paths");
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
