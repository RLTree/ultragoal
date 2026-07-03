use crate::cli::garbage::collection::parse as parse_gc;
use crate::cli::garbage::collection::types::GarbageOperation;
use crate::cli::rust::observations::{self, Probe};
use crate::cli::rust::types::RustOperation;
use crate::cli::rust::{
    RustCommand, receipt as rust_receipt, receipt_from_observations, run as rust_run,
    run_with_receipt_value as rust_run_receipt,
};
use serde_json::json;

#[test]
fn rust_receipts_cover_write_fail_and_observed_tool_branches() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let out = root.join("target/self-tests/rust-run-receipt.json");
    let code = rust_run(
        &root,
        &RustCommand {
            operation: RustOperation::Fast,
            receipt: Some(out.clone()),
        },
    )
    .expect("rust run");
    assert_eq!(code, 0);
    assert!(out.is_file());
    let parent_file = root.join("target/self-tests/rust-parent-file");
    std::fs::write(&parent_file, "not a directory").expect("parent file");
    let write_failure = rust_run_receipt(
        &RustCommand {
            operation: RustOperation::Fast,
            receipt: Some(parent_file.join("receipt.json")),
        },
        &json!({"status":"pass"}),
    );
    assert!(write_failure.is_err());

    let fail = receipt_from_observations(
        &root,
        &RustCommand {
            operation: RustOperation::CleanProof,
            receipt: None,
        },
        1,
        observations::ObservationSet {
            value: json!({"probes":[],"raw_output_is_authority":false}),
            failures: vec!["rust_devx_clean_proof_hidden_rustc_wrapper".to_string()],
        },
    )
    .expect("fail receipt");
    assert_eq!(fail["status"], "fail");
    assert_eq!(fail["claim_ceiling"], "withheld_or_blocked");
    assert_eq!(
        rust_run_receipt(
            &RustCommand {
                operation: RustOperation::CleanProof,
                receipt: None,
            },
            &fail,
        )
        .expect("fail exit"),
        1
    );

    for (operation, expected) in [
        (RustOperation::DependencyAudit, "cargo-deny"),
        (RustOperation::CoverageProve, "cargo-llvm-cov"),
        (RustOperation::Standard, "cargo-nextest"),
        (RustOperation::Watch, "watchexec-or-bacon"),
    ] {
        let receipt = rust_receipt(
            &root,
            &RustCommand {
                operation,
                receipt: None,
            },
            1,
        )
        .expect("receipt");
        assert!(
            receipt["observed_tools"]
                .as_array()
                .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected))),
            "{operation:?} missing {expected}: {receipt}"
        );
    }
}

#[test]
fn rust_observation_failures_do_not_advance_claims() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let observed = observations::collect_from_probes(
        &root,
        RustOperation::Fast,
        vec![Probe::new(
            "missing-test-probe",
            "__ultragoal_missing_probe__",
            &["--version"],
        )],
    );
    assert!(
        observed
            .failures
            .contains(&"rust_devx_required_tool_probe_failed:missing-test-probe".to_string())
    );

    let missing_root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("rust-missing-substrate");
    std::fs::create_dir_all(&missing_root).expect("root");
    let substrate = observations::collect_from_probes(&missing_root, RustOperation::Fast, vec![]);
    assert!(
        substrate.failures.iter().any(|failure| {
            failure == "rust_devx_required_substrate_missing:rust-toolchain.toml"
        })
    );
    std::fs::remove_dir_all(missing_root).expect("cleanup");
}

#[test]
fn rust_and_gc_receipt_validators_reject_substitution_edges() {
    let receipt = crate::cli::rust::receipt::surface_value_failures(
        &json!({
            "schema": crate::cli::rust::types::RUST_RECEIPT_SCHEMA,
            "status": "pass",
            "claim_ceiling": "rust_devx_observation_bound",
            "law_ids": ["rust-command-loop-authority"],
            "issuer": {"tool":"ultragoal"},
            "command": {"name":"fast"},
            "digests": {"candidate":"sha256:candidate","cargo_lock":"sha256:lock"},
            "toolchain": {"rustc_version":{}},
            "cache": {"cache_mode":"declared_local"},
            "resource_discipline": {"bounded_resources":true},
            "tool_observations": {"probes":[],"raw_output_is_authority":false},
            "observation_failures": ["raw cargo output is not enough"],
            "staleness_policy": {"invalidates_on":[]}
        }),
        "rust-command-loop-authority",
    );
    assert!(receipt.contains(&"rust_devx_pass_with_observation_failures".to_string()));

    assert_eq!(GarbageOperation::DryRun.id(), "dry_run");
    assert_eq!(GarbageOperation::Apply.id(), "apply");
    assert_eq!(
        parse_gc(&[
            "gc".to_string(),
            "apply".to_string(),
            "--plan-digest".to_string(),
            crate::digest::bytes(b"plan"),
        ])
        .expect("parse apply")
        .expect("apply command")
        .operation,
        GarbageOperation::Apply
    );
    assert!(
        parse_gc(&[
            "gc".to_string(),
            "dry-run".to_string(),
            "--plan-digest".to_string()
        ])
        .expect("parse")
        .expect("command")
        .plan_digest
        .is_none()
    );
    let verify = parse_gc(&[
        "gc".to_string(),
        "verify".to_string(),
        "--plan-digest".to_string(),
        crate::digest::bytes(b"plan"),
        "--apply-receipt-digest".to_string(),
        crate::digest::bytes(b"apply"),
    ])
    .expect("parse")
    .expect("command");
    assert_eq!(verify.operation, GarbageOperation::Verify);
    assert!(verify.plan_digest.is_some());
    assert!(verify.apply_receipt_digest.is_some());
}
