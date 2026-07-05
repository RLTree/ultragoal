use crate::cli::garbage::collection::{GarbageCommand, parse as parse_gc};
use crate::cli::rust::observations;
use crate::cli::rust::types::RustOperation;
use crate::cli::rust::{RustCommand, parse as parse_rust, receipt_from_observations};
use serde_json::json;
#[test]
fn rust_and_gc_parser_dispatch_are_authoritative() {
    let rust_args = args(&["rust", "fast"]);
    assert!(matches!(
        crate::parse_command(&rust_args).expect("rust command"),
        crate::Command::Rust(_)
    ));

    let gc_args = args(&["gc", "plan"]);
    assert!(matches!(
        crate::parse_command(&gc_args).expect("gc command"),
        crate::Command::Garbage(_)
    ));

    assert!(
        parse_rust(&["not-rust".to_string()])
            .expect("skip")
            .is_none()
    );
    assert!(parse_gc(&["not-gc".to_string()]).expect("skip").is_none());
    assert!(parse_rust(&["rust".to_string(), "workspace".to_string()]).is_err());
    assert!(
        parse_rust(&[
            "rust".to_string(),
            "fast".to_string(),
            "--receipt".to_string()
        ])
        .expect("parse dangling receipt")
        .expect("command")
        .receipt
        .is_none()
    );
    assert!(parse_gc(&["gc".to_string(), "delete".to_string()]).is_err());
}

fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

#[test]
fn command_run_routes_rust_and_gc_without_bypass() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let rust_code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command: crate::Command::Rust(RustCommand {
            operation: RustOperation::Fast,
            receipt: None,
        }),
    })
    .expect("rust run");
    assert_eq!(rust_code, 0);

    let out_rel = std::path::PathBuf::from("validation_artifacts/gc/gc-run-receipt.json");
    let out = root.join(&out_rel);
    let gc_code = crate::command_run::run_with_exit_code(crate::Args {
        root,
        command: crate::Command::Garbage(GarbageCommand {
            operation: crate::cli::garbage::collection::types::GarbageOperation::Plan,
            receipt: Some(out_rel),
            plan_digest: Some("sha256:test-plan".to_string()),
            apply_receipt_digest: None,
        }),
    })
    .expect("gc run");
    assert_eq!(gc_code, 0);
    assert!(out.is_file());
}

#[test]
fn rust_observations_cover_tool_failures_and_policy_edges() {
    let missing = observations::command("__ultragoal_missing_probe__", &["--version"]);
    assert_eq!(missing["available"], false);
    assert_eq!(missing["success"], false);

    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    for operation in [
        RustOperation::Standard,
        RustOperation::Release,
        RustOperation::DependencyAudit,
        RustOperation::CoverageProve,
        RustOperation::Watch,
    ] {
        let observed = observations::collect(&root, operation);
        assert_eq!(observed.value["operation"], operation.id());
        assert!(
            observed
                .value
                .get("probes")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|probes| probes.len() >= 4)
        );
    }

    let hidden = observations::collect_with_policy(&root, RustOperation::CleanProof, vec![], true);
    assert!(
        hidden
            .failures
            .contains(&"rust_devx_clean_proof_hidden_rustc_wrapper".to_string())
    );
}

#[test]
fn rust_operations_are_closed_over_laws_and_surfaces() {
    for (operation, id, law, surface) in [
        (
            RustOperation::ToolchainVerify,
            "toolchain_verify",
            "rust-toolchain-substrate-authority",
            "toolchain_substrate",
        ),
        (
            RustOperation::Fast,
            "fast",
            "rust-command-loop-authority",
            "fast_feedback",
        ),
        (
            RustOperation::Standard,
            "standard",
            "rust-command-loop-authority",
            "standard_repair_proof",
        ),
        (
            RustOperation::Release,
            "release",
            "rust-command-loop-authority",
            "release_loop_observation",
        ),
        (
            RustOperation::CleanProof,
            "clean_proof",
            "rust-cache-no-cache-honesty",
            "clean_checkout_no_hidden_cache",
        ),
        (
            RustOperation::Watch,
            "watch",
            "rust-command-loop-authority",
            "watch_observation",
        ),
        (
            RustOperation::MemoryProve,
            "memory_prove",
            "rust-memory-resource-discipline",
            "memory_resource_discipline",
        ),
        (
            RustOperation::DependencyAudit,
            "dependency_audit",
            "rust-developer-experience-authority",
            "dependency_supply_chain",
        ),
        (
            RustOperation::CoverageProve,
            "coverage_prove",
            "rust-command-loop-authority",
            "exact_coverage",
        ),
        (
            RustOperation::WorkspaceTopology,
            "workspace_topology",
            "rust-developer-experience-authority",
            "workspace_module_topology",
        ),
    ] {
        assert_eq!(operation.id(), id);
        assert_eq!(operation.law_id(), law);
        assert_eq!(operation.proof_surface(), surface);
    }
}

#[test]
fn rust_receipt_construction_rejects_missing_digest_inputs() {
    for missing in [
        "rust-toolchain.toml",
        "Cargo.lock",
        "Cargo.toml",
        ".cargo/config.toml",
        "schemas/schema-catalog.json",
        "docs/mandatory-law-surfaces.json",
        "templates/agent-standards/enforcement.json",
        "docs/source-obligation-matrix.json",
        "templates/RED_FIXTURES.json",
    ] {
        let root = rust_receipt_root("rust-receipt-missing-digest");
        std::fs::remove_file(root.join(missing)).expect("remove digest input");
        let err = receipt_from_observations(
            &root,
            &RustCommand {
                operation: RustOperation::Fast,
                receipt: None,
            },
            1,
            observations::ObservationSet {
                value: json!({"probes":[],"raw_output_is_authority":false}),
                failures: vec![],
            },
        )
        .expect_err("missing digest input is fail closed");
        assert!(err.contains(missing), "{missing}: {err}");
        std::fs::remove_dir_all(root).expect("cleanup digest input");
    }
}

#[test]
fn rust_run_rejects_missing_package_manifest_before_receipt_claim() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("rust-run-no-manifest");
    std::fs::create_dir_all(&root).expect("root");
    let err = crate::cli::rust::run(
        &root,
        &RustCommand {
            operation: RustOperation::Fast,
            receipt: None,
        },
    )
    .expect_err("missing package manifest rejects rust claim");
    assert!(err.contains("plugin-manifest-draft.json"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup no manifest");
}

fn rust_receipt_root(label: &str) -> std::path::PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    for dir in [".cargo", "schemas", "docs", "templates/agent-standards"] {
        std::fs::create_dir_all(root.join(dir)).expect("receipt dirs");
    }
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        r#"{"resources":[]}"#,
    )
    .expect("manifest");
    for rel in [
        "rust-toolchain.toml",
        "Cargo.lock",
        "Cargo.toml",
        ".cargo/config.toml",
        "schemas/schema-catalog.json",
        "docs/mandatory-law-surfaces.json",
        "templates/agent-standards/enforcement.json",
        "docs/source-obligation-matrix.json",
        "templates/RED_FIXTURES.json",
    ] {
        std::fs::write(root.join(rel), "x").expect("receipt file");
    }
    root
}
