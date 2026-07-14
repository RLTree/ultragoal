use crate::cli::garbage::collection::{GarbageCommand, receipt as gc_receipt};
use crate::cli::rust::observations::ObservationSet;
use crate::cli::rust::operation::RustOperation;
use crate::cli::rust::{RustCommand, receipt_from_observations};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn write_standards_rust_gc(root: &Path, current: &str) {
    copy_digest_inputs(root);
    write_standards_receipt(root, current);
    for (name, operation) in [
        ("toolchain-receipt", RustOperation::ToolchainVerify),
        ("fast-receipt", RustOperation::Fast),
        ("standard-receipt", RustOperation::Standard),
        ("release-receipt", RustOperation::Release),
        ("clean-proof-receipt", RustOperation::CleanProof),
        ("watch-receipt", RustOperation::Watch),
        ("memory-receipt", RustOperation::MemoryProve),
        ("dependency-receipt", RustOperation::DependencyAudit),
        ("coverage-receipt", RustOperation::CoverageProve),
        (
            "workspace-topology-receipt",
            RustOperation::WorkspaceTopology,
        ),
    ] {
        write_json(
            &root.join(format!("validation_artifacts/rust/{name}.json")),
            &rust_receipt(root, operation),
        );
    }
    for (name, operation) in [
        (
            "plan",
            crate::cli::garbage::collection::operation::GarbageOperation::Plan,
        ),
        (
            "dry-run",
            crate::cli::garbage::collection::operation::GarbageOperation::DryRun,
        ),
        (
            "apply",
            crate::cli::garbage::collection::operation::GarbageOperation::Apply,
        ),
        (
            "verify",
            crate::cli::garbage::collection::operation::GarbageOperation::Verify,
        ),
    ] {
        write_json(
            &root.join(format!("validation_artifacts/gc/{name}-receipt.json")),
            &gc_receipt_value(root, operation),
        );
    }
}

fn copy_digest_inputs(root: &Path) {
    let repo = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    for rel in [
        "rust-toolchain.toml",
        "Cargo.lock",
        "Cargo.toml",
        ".cargo/config.toml",
        "docs/mandatory-law-surfaces.json",
    ] {
        let dst = root.join(rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).expect("digest input parent");
        }
        std::fs::copy(repo.join(rel), dst).expect("copy digest input");
    }
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_standards_receipt(root: &Path, current: &str) {
    let artifact = |rel: &str| json!({"path":rel,"digest":crate::digest::file(&root.join(rel)).expect("artifact digest")});
    write_json(
        &root.join(
            "validation_artifacts/standards-gardener/current-standards-gardening-receipt.json",
        ),
        &json!({
            "schema":"harness-ultragoal.standards-gardening-receipt.v1",
            "status":"pass",
            "generated_at":"2030-01-01T00:00:00Z",
            "candidate_digest":current,
            "trigger_signal":{
                "signal_id":"fixture",
                "severity":"moderate",
                "signal_kind":"standards_entropy",
                "summary":"Production control fixture proves standards gardener binding",
                "source":"self-test"
            },
            "decision":{
                "accepted":true,
                "action":"validator_check",
                "rationale":"Promote standards drift to deterministic CLI evidence"
            },
            "changed_artifacts":[
                artifact("templates/agent-standards/enforcement.json"),
                artifact("docs/source-obligation-matrix.json"),
                artifact("docs/foundational-law-traceability.json"),
                artifact("templates/RED_FIXTURES.json")
            ],
            "safeguards":{"deterministic_first":true,"no_hook_by_default":true},
            "claim_ceiling":"package_static_fixture_only"
        }),
    );
}

fn rust_receipt(root: &Path, operation: RustOperation) -> Value {
    let command = RustCommand {
        operation,
        receipt: None,
    };
    receipt_from_observations(root, &command, 1, rust_observations(operation))
        .expect("rust receipt")
}

fn rust_observations(operation: RustOperation) -> ObservationSet {
    ObservationSet {
        value: json!({
            "probes":[{
                "id":"fixture",
                "required":true,
                "program":"ultragoal",
                "args":["rust", operation.id()],
                "observation":{"available":true,"success":true}
            }],
            "raw_output_is_authority":false,
            "missing_tool_is_claim_blocking":true,
            "operation":operation.id()
        }),
        failures: Vec::new(),
    }
}

fn gc_receipt_value(
    root: &Path,
    operation: crate::cli::garbage::collection::operation::GarbageOperation,
) -> Value {
    gc_receipt(
        root,
        &GarbageCommand {
            operation,
            receipt: None,
            plan_digest: Some(crate::digest::bytes(b"fixture-plan")),
            apply_receipt_digest: Some(crate::digest::bytes(b"fixture-apply")),
        },
    )
    .expect("gc receipt")
}
