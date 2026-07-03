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
            "plugin_manifest_digest": crate::self_tests::boundaries::workspace_fixtures::sha('f'),
            "plugin_name": "harness-ultragoal",
            "plugin_version": "0.0.test"
        },
        "target": {
            "surface": surface,
            "logical_path": "codex-plugin-install-harness-ultragoal",
            "path_authority": "cli_or_default_local_root_redacted",
            "exists": true,
            "package_digest": crate::self_tests::boundaries::workspace_fixtures::sha('e'),
            "package_error": null,
            "plugin_manifest_digest": crate::self_tests::boundaries::workspace_fixtures::sha('f'),
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
fn typed_status_and_package_surface_edges_are_explicit() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('d');
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
    let mut wrong_expected_install = fail_closed_install.clone();
    wrong_expected_install["target"]["expected_package_digest"] =
        json!(crate::self_tests::boundaries::workspace_fixtures::sha('e'));
    assert!(
        super::label_failures(&root, "install_audit", &wrong_expected_install, &candidate)
            .iter()
            .any(|failure| failure == "package_surface_audit_target_expected_digest_mismatch")
    );
    assert!(
        super::label_failures(&root, "install_audit", &json!({}), &candidate)
            .iter()
            .any(|failure| failure.starts_with("package_surface_audit_schema:"))
    );
    let fail_closed_cache =
        fail_closed_surface_receipt(&candidate, "cache_audit", "versioned_cache_package");
    assert!(super::label_failures(&root, "cache_audit", &fail_closed_cache, &candidate).is_empty());
    assert_eq!(
        super::typed_status("product_fitness", &json!({}), &[]),
        Some("pass")
    );
    assert_eq!(
        super::typed_status(
            "unknown",
            &json!({"status":"pass"}),
            &["ignored".to_string()]
        ),
        Some("pass")
    );
    assert_eq!(
        super::typed_status("unknown", &json!({"status":"fail"}), &[]),
        Some("fail")
    );
    assert_eq!(super::typed_status("unknown", &json!({}), &[]), None);
    assert_eq!(
        super::typed_status("unknown", &json!({}), &["failure".to_string()]),
        None
    );
    assert!(
        super::label_failures(&root, "transactional_finalization", &json!({}), &candidate)
            .iter()
            .any(|failure| failure == "cli_control_plane_transaction_wrong_schema")
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
}
