use serde_json::{Value, json};

fn fail_closed_surface_receipt(candidate: &str, operation: &str, surface: &str) -> Value {
    json!({
        "schema": crate::cli::control::plane::surface::SCHEMA,
        "schema_version": "v1",
        "issuer": {
            "tool": "ultragoal",
            "authority": "cli_control_plane",
            "compatibility_binary": "ultragoal-validator"
        },
        "generated_at": "2026-01-01T00:00:00Z",
        "root": ".",
        "operation": operation,
        "candidate_digest": candidate,
        "source": {
            "package_digest": candidate,
            "plugin_manifest_digest": crate::self_tests::boundaries::support::sha('f'),
            "plugin_name": "harness-ultragoal",
            "plugin_version": "0.0.test"
        },
        "target": {
            "surface": surface,
            "logical_path": "codex-plugin-install-harness-ultragoal",
            "path_authority": "cli_or_default_local_root_redacted",
            "exists": true,
            "package_digest": crate::self_tests::boundaries::support::sha('e'),
            "package_error": null,
            "plugin_manifest_digest": crate::self_tests::boundaries::support::sha('f'),
            "plugin_name": "harness-ultragoal",
            "plugin_version": "0.0.test",
            "expected_package_digest": candidate,
            "source_plugin_version": "0.0.test"
        },
        "status": "fail",
        "claim_ceiling": "withheld_or_blocked",
        "same_candidate": false,
        "unsupported_claim_classes": [
            "app_registry_or_reviewer_exposure",
            "plugins_ui_visibility",
            "marketplace_publication",
            "install_button_success",
            "launcher_runtime_exposure",
            "review_readiness",
            "release_readiness",
            "completion",
            "update_goal_eligibility"
        ],
        "failures": ["package_surface_digest_mismatch"],
        "blocked_claim_classes": [
            "install_cache_parity",
            "package_readiness",
            "review_readiness",
            "release_readiness",
            "completion",
            "update_goal_eligibility"
        ]
    })
}

#[test]
fn label_failures_cover_unknown_rust_and_gc_status_edges() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let candidate = crate::self_tests::boundaries::support::sha('d');
    assert_eq!(
        super::label_failures(&root, "unknown", &json!({}), &candidate),
        vec!["unknown_evidence_label:unknown"]
    );
    let rust = json!({
        "schema":"harness-ultragoal.rust-devx-receipt.v1",
        "law_id":"rust-command-loop-authority",
        "status":"fail",
        "digests":{"candidate":candidate},
        "observations":[],
        "observation_failures":[],
        "claim_ceiling":"rust_devx_observation_bound"
    });
    assert!(
        super::label_failures(&root, "rust_fast", &rust, &candidate)
            .iter()
            .any(|failure| failure == "rust_receipt_status_not_pass")
    );
    let source_audit = json!({
        "status":"fail",
        "target_revision":{"kind":"package_digest","value":candidate}
    });
    assert!(
        super::label_failures(&root, "source_audit", &source_audit, &candidate)
            .iter()
            .any(|failure| failure == "source_audit_status_not_pass")
    );
    let coverage_wrong_target = json!({
        "target_revision":{"kind":"package_digest","value":crate::self_tests::boundaries::support::sha('e')},
        "coverage":{"percent":100.0},
        "uncovered_records":[],
        "claim_ceiling":"supports_complete_claim"
    });
    assert!(
        super::label_failures(&root, "coverage", &coverage_wrong_target, &candidate)
            .iter()
            .any(|failure| failure == "coverage_target_digest_mismatch")
    );
    let coverage_pass = json!({
        "target_revision":{"kind":"package_digest","value":candidate},
        "coverage":{"percent":100.0},
        "uncovered_records":[],
        "claim_ceiling":"supports_complete_claim"
    });
    assert!(super::label_failures(&root, "coverage", &coverage_pass, &candidate).is_empty());
    let coverage_not_exact = json!({
        "target_revision":{"kind":"package_digest","value":candidate},
        "coverage":{"percent":99.0},
        "uncovered_records":["validator/src/main.rs:1"],
        "claim_ceiling":"supports_complete_claim"
    });
    assert!(
        super::label_failures(&root, "coverage", &coverage_not_exact, &candidate)
            .iter()
            .any(|failure| failure == "coverage_not_exact_100")
    );
    let coverage_with_uncovered_records = json!({
        "target_revision":{"kind":"package_digest","value":candidate},
        "coverage":{"percent":100.0},
        "uncovered_records":["validator/src/main.rs:1"],
        "claim_ceiling":"supports_complete_claim"
    });
    assert!(
        super::label_failures(
            &root,
            "coverage",
            &coverage_with_uncovered_records,
            &candidate
        )
        .iter()
        .any(|failure| failure == "coverage_not_exact_100")
    );
    let red_fixture_report = json!({
        "status":"pass",
        "target_revision":{"kind":"package_digest","value":candidate}
    });
    assert!(
        super::label_failures(&root, "red_fixture_report", &red_fixture_report, &candidate)
            .is_empty()
    );
    assert!(
        super::label_failures(
            &root,
            "install_audit",
            &json!({"schema":"wrong"}),
            &candidate
        )
        .iter()
        .any(|failure| failure.starts_with("package_surface_audit_schema:"))
    );
    let fail_closed_install =
        fail_closed_surface_receipt(&candidate, "install_audit", "installed_plugin");
    assert!(
        super::label_failures(&root, "install_audit", &fail_closed_install, &candidate).is_empty()
    );
    let fail_closed_cache =
        fail_closed_surface_receipt(&candidate, "cache_audit", "versioned_cache_package");
    assert!(super::label_failures(&root, "cache_audit", &fail_closed_cache, &candidate).is_empty());
    assert_eq!(
        super::typed_status("product_fitness", &json!({}), &[]),
        Some("pass")
    );
    for label in [
        "coverage",
        "fit_repo",
        "product_fitness",
        "standards_gardener",
    ] {
        assert_eq!(super::typed_status(label, &json!({}), &[]), Some("pass"));
        assert_eq!(
            super::typed_status(label, &json!({}), &["not_current".to_string()]),
            None
        );
    }
    let gc = json!({
        "schema":"harness-ultragoal.workspace-gc-receipt.v1",
        "law_id":"workspace-artifact-cache-garbage-collection",
        "status":"fail",
        "digests":{"candidate":candidate},
        "plan":{"id":"plan"},
        "observations":[],
        "observation_failures":[],
        "claim_ceiling":"gc_observation_bound"
    });
    assert!(
        super::label_failures(&root, "gc_plan", &gc, &candidate)
            .iter()
            .any(|failure| failure == "gc_receipt_status_not_pass")
    );
}
