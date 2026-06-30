use serde_json::{Value, json};

fn valid_manifest(root: &std::path::Path) -> Value {
    for rel in [
        ".harness/coverage-command",
        "scripts/check",
        "validator/src/main.rs",
        "schemas/example.schema.json",
        "templates/example.md",
        "skills/ultragoal/SKILL.md",
        "agents/product.md",
        "custom-agents/product.toml",
        "docs/example.md",
    ] {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(path, format!("{rel}\n")).expect("fixture file");
    }
    let mut manifest = json!({
        "schema": "harness-ultragoal.coverage-manifest.v1",
        "coverage_command_path": ".harness/coverage-command",
        "repo_root_digest": crate::digest::ZERO,
        "required_target_paths": [".harness", "scripts", "validator", "schemas", "templates", "skills", "agents", "custom-agents", "docs"],
        "repo_owned_source_roots": [".harness", "scripts", "validator", "schemas", "templates", "skills", "agents", "custom-agents", "docs"],
        "changed_file_coupling_policy": {
            "required": true,
            "changed_files": ["validator/src/main.rs"],
            "changed_files_digest": crate::digest::ZERO
        },
        "required_measured_dimensions_per_root": [{
            "root": "validator",
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
    });
    let source = crate::claim_semantics::coverage::digests::source_tree_digest(root, &manifest)
        .expect("source digest");
    let changed = crate::claim_semantics::coverage::digests::changed_files_digest(root, &manifest)
        .expect("changed digest");
    manifest["repo_root_digest"] = json!(source);
    manifest["changed_file_coupling_policy"]["changed_files_digest"] = json!(changed);
    manifest
}

#[test]
fn coverage_scope_accepts_current_digest_authority_and_rejects_empty_sets() {
    let root = crate::self_tests::boundaries::support::temp_root("coverage-scope-current");
    let manifest = valid_manifest(&root);
    let failures = crate::audit::coverage::scope::value_failures_with_root(&manifest, Some(&root));
    let failure_text = failures.join("\n");
    assert!(
        !failure_text.contains("coverage_receipt_source_digest_mismatch"),
        "{failures:?}"
    );
    assert!(
        !failure_text.contains("coverage_receipt_changed_files_digest_mismatch"),
        "{failures:?}"
    );

    let mut empty = manifest;
    empty["required_target_paths"] = json!([]);
    empty["required_measured_dimensions_per_root"] = json!([]);
    let empty_failures =
        crate::audit::coverage::scope::value_failures_with_root(&empty, Some(&root));
    assert!(empty_failures.contains(&"coverage_claim_missing_target_paths".to_string()));
    assert!(empty_failures.contains(&"coverage_claim_missing_dimensions".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup coverage scope current");
}

#[test]
fn coverage_scope_subchecks_reject_missing_changed_files_and_weak_scripts() {
    let root = crate::self_tests::boundaries::support::temp_root("coverage-scope-subchecks");
    std::fs::create_dir_all(&root).expect("coverage root");
    let no_root_failures = crate::audit::coverage::scope::changed_files::failures(
        None,
        &json!({"changed_files":["src/lib.rs"]}),
    );
    assert!(no_root_failures.is_empty());
    let changed_failures = crate::audit::coverage::scope::changed_files::failures(
        Some(&root),
        &json!({"changed_files":[7, "../escape.rs", "missing.rs"]}),
    );
    assert_eq!(
        changed_failures,
        vec![
            "coverage_changed_file_missing_from_manifest",
            "coverage_changed_file_missing_from_manifest",
            "coverage_changed_file_missing_from_manifest"
        ]
    );
    let generated = root.join("validation_artifacts/coverage/coverage-receipt.json");
    std::fs::create_dir_all(generated.parent().expect("generated parent")).expect("generated dir");
    std::fs::write(&generated, "{}").expect("generated receipt");
    let generated_failures = crate::audit::coverage::scope::changed_files::failures(
        Some(&root),
        &json!({"changed_files":["validation_artifacts/coverage/coverage-receipt.json"]}),
    );
    assert_eq!(
        generated_failures,
        vec!["coverage_changed_file_generated_artifact"]
    );
    let missing_script =
        crate::audit::coverage::scope::scripts::failures(&root, "scripts/missing", "completion");
    assert!(missing_script.contains(&"coverage_scope_surface_missing:scripts/missing".to_string()));
    std::fs::create_dir_all(root.join("scripts")).expect("scripts dir");
    std::fs::write(
        root.join("scripts/check-coverage-fast"),
        "progress claim only\n",
    )
    .expect("script");
    let completion_failures = crate::audit::coverage::scope::scripts::failures(
        &root,
        "scripts/check-coverage-fast",
        "completion",
    );
    assert!(completion_failures.contains(&"coverage_full_gate_missing".to_string()));
    assert!(completion_failures.contains(&"coverage_fast_gate_used_for_completion".to_string()));
    let progress_failures = crate::audit::coverage::scope::scripts::failures(
        &root,
        "scripts/check-coverage-fast",
        "progress",
    );
    assert!(
        !progress_failures.contains(&"coverage_claim_context_missing".to_string()),
        "{progress_failures:?}"
    );
    std::fs::write(root.join("scripts/check-coverage-fast"), "no context\n").expect("script");
    assert!(
        crate::audit::coverage::scope::scripts::failures(
            &root,
            "scripts/check-coverage-fast",
            "progress"
        )
        .contains(&"coverage_claim_context_missing".to_string())
    );
    std::fs::remove_dir_all(root).expect("cleanup coverage scope subchecks");
}

#[test]
fn coverage_digests_ignore_manifest_surfaces_and_prefix_patterns() {
    let root = crate::self_tests::boundaries::support::temp_root("coverage-digest-ignore");
    for rel in [
        "src/lib.rs",
        "src/generated/out.rs",
        "docs/parent-session-full-ultragoal-compliance-checklist-2026-06-25.md",
        ".harness/coverage-command",
        ".harness/coverage-manifest.json",
        "templates/.harness/coverage-command",
        "templates/.harness/coverage-manifest.json",
    ] {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        std::fs::write(path, rel).expect("fixture file");
    }
    let manifest = json!({
        "required_target_paths":["src", "docs", ".harness", "templates/.harness"],
        "source_discovery_rules":{"ignore":["src/generated/**"]},
        "changed_file_coupling_policy":{"changed_files":["src/lib.rs"]}
    });
    let before = crate::claim_semantics::coverage::digests::source_tree_digest(&root, &manifest)
        .expect("source digest");
    std::fs::write(
        root.join("src/generated/out.rs"),
        "ignored generated change",
    )
    .expect("generated change");
    std::fs::write(
        root.join(".harness/coverage-command"),
        "ignored command change",
    )
    .expect("command change");
    std::fs::write(
        root.join("templates/.harness/coverage-manifest.json"),
        "ignored template manifest change",
    )
    .expect("template manifest change");
    std::fs::write(
        root.join("docs/parent-session-full-ultragoal-compliance-checklist-2026-06-25.md"),
        "ignored progress update",
    )
    .expect("checklist change");
    let after_ignored =
        crate::claim_semantics::coverage::digests::source_tree_digest(&root, &manifest)
            .expect("source digest after ignored");
    assert_eq!(before, after_ignored);

    std::fs::write(root.join("src/lib.rs"), "included change").expect("included change");
    let after_included =
        crate::claim_semantics::coverage::digests::source_tree_digest(&root, &manifest)
            .expect("source digest after included");
    assert_ne!(before, after_included);
    std::fs::remove_dir_all(root).expect("cleanup coverage digest ignore");
}
