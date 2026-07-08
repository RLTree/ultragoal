use crate::cli::garbage::collection::types::GarbageOperation;
use crate::cli::garbage::collection::{GarbageCommand, receipt as gc_receipt};
use crate::cli::rust::types::RustOperation;
use crate::cli::rust::{RustCommand, parse, receipt as rust_receipt};
use serde_json::json;

#[test]
fn rust_command_parser_routes_developer_workflow_operations() {
    for (raw, expected) in [
        (
            vec!["rust", "toolchain", "verify"],
            RustOperation::ToolchainVerify,
        ),
        (vec!["rust", "fast"], RustOperation::Fast),
        (vec!["rust", "standard"], RustOperation::Standard),
        (vec!["rust", "release"], RustOperation::Release),
        (vec!["rust", "clean-proof"], RustOperation::CleanProof),
        (vec!["rust", "watch"], RustOperation::Watch),
        (vec!["rust", "memory", "prove"], RustOperation::MemoryProve),
        (
            vec!["rust", "dependency", "audit"],
            RustOperation::DependencyAudit,
        ),
        (
            vec!["rust", "coverage", "prove"],
            RustOperation::CoverageProve,
        ),
        (
            vec!["rust", "workspace", "topology", "check"],
            RustOperation::WorkspaceTopology,
        ),
    ] {
        let args = raw.into_iter().map(str::to_string).collect::<Vec<_>>();
        let command = parse(&args).expect("parse").expect("rust command");
        assert_eq!(command.operation, expected);
    }
}

#[test]
fn rust_command_usage_matches_developer_workflow_surface() {
    let usage = crate::usage();
    for fragment in [
        "rust <toolchain verify",
        "memory prove",
        "dependency audit",
        "coverage prove --exact",
        "workspace topology check",
    ] {
        assert!(usage.contains(fragment), "{fragment}");
    }
    for invalid in ["rust <toolchain|", "memory|dependency", "workspace>"] {
        assert!(!usage.contains(invalid), "{invalid}");
    }
}

#[test]
fn rust_devx_receipt_binds_cli_authority_and_rejects_wrong_law() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let command = RustCommand {
        operation: RustOperation::ToolchainVerify,
        receipt: None,
    };
    let receipt = rust_receipt(&root, &command, 7).expect("rust receipt");
    assert_eq!(receipt["issuer"]["tool"], "ultragoal");
    assert_eq!(receipt["command"]["raw_tools_are_observations_only"], true);
    assert_eq!(
        receipt["tool_observations"]["raw_output_is_authority"],
        false
    );
    assert!(
        receipt["tool_observations"]["probes"]
            .as_array()
            .is_some_and(|items| items.len() >= 4)
    );
    assert!(
        crate::cli::rust::receipt::surface_value_failures(
            &receipt,
            "rust-toolchain-substrate-authority"
        )
        .is_empty()
    );
    let serialized = serde_json::to_string(&receipt).expect("serialize rust receipt");
    assert!(
        !serialized.contains(users_marker()),
        "rust receipts must not package private home paths: {serialized}"
    );
    let wrong = crate::cli::rust::receipt::surface_value_failures(
        &receipt,
        "rust-memory-resource-discipline",
    );
    assert!(
        wrong.contains(
            &"rust_devx_receipt_missing_law_id:rust-memory-resource-discipline".to_string()
        )
    );

    let mut theater = receipt;
    theater["tool_observations"]["raw_output_is_authority"] = json!(true);
    let failures = crate::cli::rust::receipt::surface_value_failures(
        &theater,
        "rust-toolchain-substrate-authority",
    );
    assert!(failures.contains(&"rust_devx_raw_output_marked_authority".to_string()));
}

#[test]
fn rust_observation_excerpts_redact_private_home_paths() {
    let excerpt = crate::cli::rust::observations::redacted_excerpt_for_test(&format!(
        "1.95.0-aarch64-apple-darwin (overridden by '{}tree/project/rust-toolchain.toml')",
        users_marker()
    ));
    assert_eq!(
        excerpt,
        "1.95.0-aarch64-apple-darwin (overridden by '[redacted-home-path]')"
    );

    let metadata = crate::cli::rust::observations::redacted_excerpt_for_test(&format!(
        r#"{{"id":"path+file://{}tree/project/validator#ultragoal@0.1.0"}}"#,
        users_marker()
    ));
    assert!(!metadata.contains(users_marker()), "{metadata}");
    assert!(metadata.contains("[redacted-home-path]"), "{metadata}");
}

fn users_marker() -> &'static str {
    concat!("/", "Users/")
}

#[test]
fn workspace_gc_receipt_binds_plan_and_rejects_missing_law() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let command = GarbageCommand {
        operation: GarbageOperation::Verify,
        receipt: None,
        plan_digest: Some(crate::digest::bytes(b"plan")),
        apply_receipt_digest: Some(crate::digest::bytes(b"apply")),
    };
    let receipt = gc_receipt(&root, &command).expect("gc receipt");
    assert_eq!(receipt["issuer"]["authority"], "cli_control_plane");
    assert_eq!(receipt["deletion_plan"]["blind_rm_rf_allowed"], false);
    assert!(crate::cli::garbage::collection::receipt::surface_value_failures(&receipt).is_empty());

    let mut weak_required_args = receipt.clone();
    weak_required_args["deletion_plan"]["plan_digest_source"] = json!("derived_from_plan");
    weak_required_args["deletion_plan"]["plan_digest_argument_required"] = json!(false);
    weak_required_args["post_verify"]["apply_receipt_digest_required"] = json!(false);
    let failures =
        crate::cli::garbage::collection::receipt::surface_value_failures(&weak_required_args);
    for expected in [
        "workspace_gc_receipt_plan_digest_not_from_required_argument",
        "workspace_gc_receipt_plan_digest_argument_not_required",
        "workspace_gc_receipt_apply_digest_not_required",
    ] {
        assert!(
            failures.contains(&expected.to_string()),
            "{expected}: {failures:?}"
        );
    }

    let mut missing = receipt;
    missing["law_ids"] = json!([]);
    let failures = crate::cli::garbage::collection::receipt::surface_value_failures(&missing);
    assert!(failures.contains(&"workspace_gc_receipt_missing_law_id".to_string()));

    let missing_apply = gc_receipt(
        &root,
        &GarbageCommand {
            operation: GarbageOperation::Verify,
            receipt: None,
            plan_digest: Some(crate::digest::bytes(b"plan")),
            apply_receipt_digest: None,
        },
    )
    .expect("missing apply receipt");
    assert_eq!(missing_apply["status"], "fail");
    assert!(
        missing_apply["observation_failures"]
            .as_array()
            .expect("gc failures")
            .iter()
            .any(|failure| failure.as_str() == Some("workspace_gc_apply_receipt_digest_missing"))
    );

    let mut forged_pass = missing_apply;
    forged_pass["status"] = json!("pass");
    forged_pass["claim_ceiling"] = json!("gc_observation_bound");
    let failures = crate::cli::garbage::collection::receipt::surface_value_failures(&forged_pass);
    assert!(failures.contains(&"workspace_gc_receipt_pass_with_observation_failures".to_string()));
}

#[test]
fn rust_devx_audit_fails_closed_when_surfaces_are_absent() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("rust-devx-audit");
    std::fs::create_dir_all(&root).expect("root");
    let failures = crate::audit::rust::developer::package_failures(&root);
    assert!(failures.iter().any(|(law, failure)| {
        law == "rust-toolchain-substrate-authority"
            && failure.starts_with("rust_devx_missing_artifact")
    }));
    assert!(failures.iter().any(|(law, failure)| {
        law == "workspace-artifact-cache-garbage-collection"
            && failure.starts_with("rust_devx_missing_artifact")
    }));
    std::fs::remove_dir_all(root).expect("cleanup");
}
