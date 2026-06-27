use serde_json::json;

#[test]
fn rust_devx_audit_receipt_edges_are_typed() {
    let rust_law = &crate::audit::rust::developer::LAWS[0];
    let rust_failures = crate::audit::rust::developer::receipt_failures(
        &json!({
            "status":"pass",
            "law_ids":[rust_law.id],
            "digests":{"candidate":"sha256:old"},
            "issuer":{"tool":"ultragoal","authority":"cli_control_plane"},
            "command":{"raw_tools_are_observations_only":true},
            "observation_failures":[]
        }),
        rust_law,
        "sha256:new",
    );
    assert!(rust_failures.contains(&"rust_devx_receipt_candidate_digest_mismatch".to_string()));

    let gc_law = crate::audit::rust::developer::LAWS
        .iter()
        .find(|law| law.id == "workspace-artifact-cache-garbage-collection")
        .expect("gc law");
    let gc_failures = crate::audit::rust::developer::receipt_failures(
        &json!({
            "status":"pass",
            "law_ids":[gc_law.id],
            "digests":{"candidate":"sha256:old"},
            "issuer":{"tool":"ultragoal","authority":"cli_control_plane"},
            "deletion_plan":{"blind_rm_rf_allowed":false}
        }),
        gc_law,
        "sha256:new",
    );
    assert!(gc_failures.contains(&"workspace_gc_receipt_candidate_digest_mismatch".to_string()));
    assert!(
        crate::audit::rust::developer::receipt_failures(
            &json!({
                "schema": crate::cli::rust::types::RUST_RECEIPT_SCHEMA,
                "status":"fail",
                "claim_ceiling":"withheld_or_blocked",
                "law_ids":[rust_law.id],
                "digests":{"candidate":"sha256:same","cargo_lock":"sha256:lock"},
                "issuer":{"tool":"ultragoal","authority":"cli_control_plane"},
                "command":{"name":"dependency_audit","raw_tools_are_observations_only":true},
                "toolchain":{"rustc_version":{}},
                "cache":{"cache_mode":"declared_local"},
                "resource_discipline":{"bounded_resources":true},
                "tool_observations":{"probes":[],"raw_output_is_authority":false},
                "observation_failures":[],
                "staleness_policy":{"invalidates_on":[]}
            }),
            rust_law,
            "sha256:same"
        )
        .is_empty()
    );
    let gc_same = crate::audit::rust::developer::receipt_failures(
        &json!({
            "schema": crate::cli::garbage::collection::types::GC_RECEIPT_SCHEMA,
            "status":"pass",
            "law_ids":[gc_law.id],
            "digests":{"candidate":"sha256:same"},
            "issuer":{"tool":"ultragoal","authority":"cli_control_plane"},
            "deletion_plan":{"blind_rm_rf_allowed":false},
            "protected_set":{"protected_delete_allowed_without_replacement":false},
            "artifact_classification":{"unclassified_delete_allowed":false}
        }),
        gc_law,
        "sha256:same",
    );
    assert!(!gc_same.contains(&"workspace_gc_receipt_candidate_digest_mismatch".to_string()));
    assert!(
        crate::audit::rust::developer::required_schemas(rust_law)
            .contains(&"schemas/rust-devx-receipt.schema.json")
    );
    assert!(
        crate::audit::rust::developer::required_schemas(gc_law)
            .contains(&"schemas/workspace-gc-receipt.schema.json")
    );
}

#[test]
fn rust_devx_audit_law_id_lookup_accepts_obligation_id_rows() {
    let rust_law = &crate::audit::rust::developer::LAWS[0];
    let root = crate::self_tests::boundaries::support::repo_root();
    assert!(crate::audit::rust::developer::has_law_id(
        &root,
        "docs/source-obligation-matrix.json",
        "obligations",
        rust_law.id
    ));
    let temp = crate::self_tests::boundaries::support::temp_root("rust-law-obligation-id");
    std::fs::create_dir_all(temp.join("docs")).expect("docs");
    std::fs::write(
        temp.join("docs/laws.json"),
        serde_json::to_vec(&json!({"rows":[{"obligation_id":"law-from-obligation"}]}))
            .expect("laws"),
    )
    .expect("write laws");
    assert!(crate::audit::rust::developer::has_law_id(
        &temp,
        "docs/laws.json",
        "rows",
        "law-from-obligation"
    ));
    std::fs::remove_dir_all(temp).expect("cleanup obligation id");
}

#[test]
fn rust_devx_audit_live_root_covers_current_surface_presence() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let failures = crate::audit::rust::developer::package_failures(&root);
    for missing_prefix in [
        "rust_devx_missing_artifact:",
        "rust_devx_package_inventory_missing:",
        "rust_devx_schema_catalog_missing:",
        "rust_devx_missing_standards_row:",
        "rust_devx_missing_source_obligation:",
        "rust_devx_missing_foundational_trace:",
        "rust_devx_missing_valid_fixture:",
        "rust_devx_missing_red_fixture:",
        "rust_devx_missing_receipt:",
    ] {
        assert!(
            !failures
                .iter()
                .any(|(_, failure)| failure.starts_with(missing_prefix)),
            "{missing_prefix}: {failures:?}"
        );
    }
}

#[test]
fn rust_devx_audit_law_id_lookup_fails_closed_for_missing_rows() {
    let temp = crate::self_tests::boundaries::support::temp_root("rust-law-id-missing");
    std::fs::create_dir_all(temp.join("docs")).expect("docs");
    std::fs::write(temp.join("docs/empty.json"), br#"{"rows":[]}"#).expect("empty rows");
    std::fs::write(
        temp.join("docs/wrong.json"),
        br#"{"rows":[{"id":"other-law"},{"obligation_id":"other-obligation"}]}"#,
    )
    .expect("wrong rows");
    assert!(!crate::audit::rust::developer::has_law_id(
        &temp,
        "docs/missing.json",
        "rows",
        "rust-command-loop-authority"
    ));
    assert!(!crate::audit::rust::developer::has_law_id(
        &temp,
        "docs/empty.json",
        "rows",
        "rust-command-loop-authority"
    ));
    assert!(!crate::audit::rust::developer::has_law_id(
        &temp,
        "docs/wrong.json",
        "rows",
        "rust-command-loop-authority"
    ));
    std::fs::remove_dir_all(temp).expect("cleanup missing law id");
}
