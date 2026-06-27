use serde_json::json;

#[test]
fn anti_theater_laws_join_to_final_packet_registry_and_cli_authority() {
    let root = crate::self_tests::boundaries::support::temp_root("anti-theater-deps");
    crate::self_tests::audit::final_packet::support::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    let law = "generated-proof-artifact-provenance-anti-fabrication";
    let missing = crate::audit::mandatory::law::surfaces::anti_theater_dependency_failures_for_test(
        &root, &store, law,
    );
    for expected in [
        "final_packet_proof_missing",
        "plugin_self_law_json_missing_or_malformed",
        "cli_control_plane_receipt_missing",
    ] {
        assert!(
            missing.iter().any(|failure| failure.contains(expected)),
            "{expected}: {missing:?}"
        );
    }

    let current = crate::package::inventory::package_digest(&root).expect("digest");
    crate::self_tests::audit::final_packet::support::write_green_proof(&root, &current);
    let failures =
        crate::audit::mandatory::law::surfaces::anti_theater_dependency_failures_for_test(
            &root, &store, law,
        );
    assert!(failures.is_empty(), "{failures:?}");
    std::fs::remove_dir_all(root).expect("cleanup anti-theater deps");
}
