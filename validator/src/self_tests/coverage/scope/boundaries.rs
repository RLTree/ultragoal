use serde_json::json;

#[test]
fn coverage_scope_rootless_valid_digests_do_not_create_stale_receipt_failures() {
    let failures = crate::audit::coverage::scope::value_failures_with_root(
        &json!({
            "schema": "harness-ultragoal.coverage-manifest.v1",
            "coverage_command_path": ".harness/coverage-command",
            "repo_root_digest": crate::self_tests::boundaries::support::sha('a'),
            "required_target_paths": ["src", "scripts", "validator", "schemas", "templates"],
            "repo_owned_source_roots": ["src", "scripts", "validator", "schemas", "templates"],
            "changed_file_coupling_policy": {
                "required": true,
                "changed_files": ["src/lib.rs"],
                "changed_files_digest": crate::self_tests::boundaries::support::sha('b')
            },
            "required_measured_dimensions_per_root": [{
                "root": "src",
                "dimensions": ["line", "branch", "function", "artifact", "ui_state"]
            }],
            "repo_walk_policy": {
                "classify_all_nonignored_files": true,
                "ignored_local_state_cannot_be_target": true
            },
            "policy_mutation_gate": {"material_review_required": true},
            "receipt_freshness_binding": {
                "source_tree_digest_required": true,
                "manifest_digest_required": true,
                "command_digest_required": true,
                "changed_files_digest_required": true
            },
            "tool_generated_proof_policy": {
                "hand_written_receipts_rejected": true,
                "prose_percent_rejected": true
            },
            "fast_full_gate_split": {
                "full_required_for_completion": true,
                "fast_supports_completion": false
            },
            "behavior_dimension_mapping": {
                "cli_tooling": ["line"],
                "library_api": ["line"],
                "branching_error_behavior": ["branch"],
                "product_ui_control_surface": ["ui_state"],
                "generated_authority": ["artifact"],
                "workflow_orchestration": ["function"],
                "security_trust_boundary": ["branch"],
                "install_cache_package": ["artifact"]
            }
        }),
        None,
    );
    assert!(!failures.contains(&"coverage_receipt_source_digest_mismatch".to_string()));
    assert!(!failures.contains(&"coverage_receipt_changed_files_digest_mismatch".to_string()));
}

#[test]
fn coverage_scope_package_failures_report_malformed_manifest_before_substitution() {
    let root =
        crate::self_tests::boundaries::support::temp_root("coverage-scope-package-malformed");
    for rel in [
        "templates/.harness/coverage-command",
        "templates/scripts/check-coverage-fast",
        "templates/scripts/check-coverage-full",
        "templates/COVERAGE_RECEIPT.json",
    ] {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        std::fs::write(path, "coverage command\n").expect("surface");
    }
    let manifest = root.join("templates/.harness/coverage-manifest.json");
    std::fs::write(&manifest, "{").expect("bad manifest");
    let failures = crate::audit::coverage::scope::package_failures(&root);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("coverage_manifest_malformed")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup coverage package malformed");
}
