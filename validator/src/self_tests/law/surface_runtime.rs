use serde_json::json;
use std::path::Path;

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, text).expect("write text");
}

fn has(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}

#[test]
fn runtime_live_transcript_and_workflow_receipts_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("law-surface-receipts");
    write_text(&root.join("artifact.txt"), "artifact");
    let runtime = json!({
        "schema": "wrong",
        "tool_identity": {"version": "stale"},
        "workspace": "/tmp/other",
        "artifact_digests": [{}, {"path": "artifact.txt"}, {"path": "missing.txt", "digest": crate::self_tests::boundaries::workspace_fixtures::sha('1')}],
        "claim_ceiling": "broad"
    });
    let runtime_failures =
        crate::audit::law::surface::receipt::runtime::runtime_tool_identity_value_failures(
            &root, &runtime,
        );
    for expected in [
        "runtime_tool_identity_wrong_schema",
        "runtime_tool_identity_missing_tool",
        "runtime_tool_identity_stale_version",
        "runtime_tool_identity_missing_binary_path",
        "runtime_tool_identity_wrong_workspace",
        "runtime_tool_identity_digest_mismatch",
        "runtime_tool_identity_claim_ceiling_not_same_surface",
    ] {
        assert!(
            has(&runtime_failures, expected),
            "{expected}: {runtime_failures:?}"
        );
    }

    let live = json!({"schema":"wrong", "evidence":[{}], "claim_ceiling":"wide"});
    let live_failures =
        crate::audit::law::surface::receipt::runtime::product_live_surface_value_failures(
            &root, &live,
        );
    assert!(has(&live_failures, "product_live_surface_wrong_schema"));
    assert!(has(&live_failures, "product_live_surface_digest_mismatch"));
    assert!(has(
        &live_failures,
        "product_live_surface_substitute_not_rejected:install_success"
    ));
    assert!(has(
        &live_failures,
        "product_live_surface_claim_ceiling_not_same_surface"
    ));

    let transcript = json!({"schema":"wrong", "cleanup_complete": false});
    let transcript_failures =
        crate::audit::law::surface::receipt::runtime::transcript_quality_value_failures(
            &root,
            &transcript,
        );
    for expected in [
        "transcript_quality_wrong_schema",
        "transcript_quality_finalization_disabled",
        "transcript_quality_cleanup_incomplete",
        "transcript_quality_alignment_incomplete",
        "transcript_quality_claim_ceiling_not_supported",
    ] {
        assert!(
            has(&transcript_failures, expected),
            "{expected}: {transcript_failures:?}"
        );
    }

    let clean_checkout = json!({
        "schema": "wrong",
        "source_install_cache_command_alignment": "drift",
        "claim_ceiling": "wide",
        "commands": [{
            "id": "root",
            "command": "see docs",
            "status": "failed",
            "requires_local_author_memory": true,
            "artifacts": [{}]
        }]
    });
    let clean_failures =
        crate::audit::law::surface::receipt::workflow::clean_checkout_value_failures(
            &root,
            &clean_checkout,
        );
    for expected in [
        "clean_checkout_wrong_schema",
        "clean_checkout_missing_root_check",
        "clean_checkout_missing_installed_command",
        "clean_checkout_source_install_cache_command_drift",
        "clean_checkout_claim_ceiling_not_discoverable",
        "clean_checkout_prose_only_command:root",
        "clean_checkout_non_runnable_command:root",
        "clean_checkout_local_state_dependency:root",
        "clean_checkout_digest_mismatch",
    ] {
        assert!(
            has(&clean_failures, expected),
            "{expected}: {clean_failures:?}"
        );
    }
    assert!(has(
        &crate::audit::law::surface::receipt::workflow::clean_checkout_value_failures(
            &root,
            &json!({"commands":[]})
        ),
        "clean_checkout_missing_commands"
    ));
    std::fs::remove_dir_all(root).expect("cleanup law surface receipts");
}
