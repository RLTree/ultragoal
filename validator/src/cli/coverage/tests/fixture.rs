use super::*;
use serde_json::{Value, json};
use std::fs;

pub(super) fn write_coverage_root(root: &Path, percent: f64, uncovered: Value) {
    for relative in [".harness", "src", "validation_artifacts/coverage"] {
        fs::create_dir_all(root.join(relative)).expect("dir");
    }
    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({}),
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), "pub fn answer() -> usize { 42 }\n").expect("source");
    fs::write(
        root.join(".harness/coverage-command"),
        "bash .harness/run-coverage.sh\n",
    )
    .expect("coverage command");
    let manifest = manifest();
    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join(".harness/coverage-manifest.json"),
        &manifest,
    )
    .expect("coverage manifest");
    let report = json!({"data":[{"totals":{"lines":{"percent":percent}}}]});
    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join("validation_artifacts/coverage/llvm-cov-full.json"),
        &report,
    )
    .expect("report");
    let candidate = crate::package::inventory::package_digest(root).expect("candidate");
    let receipt = coverage_receipt(root, &manifest, percent, uncovered, &candidate);
    crate::self_tests::boundaries::workspace_fixtures::write_json(
        &root.join(COVERAGE_RECEIPT_REL),
        &receipt,
    )
    .expect("coverage receipt");
}

fn manifest() -> Value {
    json!({
        "schema": "harness-ultragoal.coverage-manifest.v1",
        "manifest_version": 1,
        "repo_root_digest": crate::digest::ZERO,
        "generated_at": "2026-01-01T00:00:00Z",
        "owner": "coverage-test-owner",
        "policy": "100_percent_required",
        "coverage_command_id": "coverage-test-command",
        "coverage_command_path": ".harness/coverage-command",
        "coverage_receipt_output_path": COVERAGE_RECEIPT_REL,
        "required_target_paths": ["src/lib.rs"],
        "changed_file_coupling_policy": {
            "required": true,
            "changed_files": ["src/lib.rs"],
            "changed_files_digest": crate::digest::ZERO,
            "blocker_path": "VERIFICATION_BACKLOG.json"
        },
        "required_measured_dimensions_per_root": [{
            "root": "src",
            "dimensions": ["line"]
        }],
        "source_discovery_rules": {"include": ["**/*"], "ignore": []},
        "repo_owned_source_roots": ["src"],
        "generated_roots": [],
        "vendor_roots": [],
        "external_roots": [],
        "exclusions": [],
        "repo_walk_policy": {},
        "behavior_dimension_mapping": {},
        "policy_mutation_gate": {},
        "receipt_freshness_binding": {},
        "tool_generated_proof_policy": {},
        "fast_full_gate_split": {
            "fast_gate": "scripts/check-coverage-fast",
            "full_gate": "scripts/check-coverage-full",
            "fast_supports_completion": false,
            "full_required_for_completion": true,
            "missing_claim_context_fails": true
        },
        "claim_ceiling_when_incomplete": "withheld_or_blocked"
    })
}

pub(super) fn coverage_receipt(
    root: &Path,
    manifest: &Value,
    percent: f64,
    uncovered: Value,
    candidate: &str,
) -> Value {
    let report = root.join("validation_artifacts/coverage/llvm-cov-full.json");
    json!({
        "schema": "harness-ultragoal.coverage-receipt.v1",
        "claim_id": "CLAIM-001",
        "command": "bash .harness/run-coverage.sh",
        "tool": "cargo-llvm-cov",
        "coverage_target_dir": "target/ultragoal-coverage",
        "source_tree_digest": crate::audit::coverage::scope::digests::source_tree_digest(root, manifest).expect("source"),
        "coverage_manifest_digest": crate::digest::file(&root.join(".harness/coverage-manifest.json")).expect("manifest digest"),
        "coverage_command_digest": crate::digest::file(&root.join(".harness/coverage-command")).expect("command digest"),
        "changed_files_digest": crate::audit::coverage::scope::digests::changed_files_digest(root, manifest).expect("changed"),
        "tool_version": "cargo-llvm-cov test",
        "workspace_root": root.canonicalize().expect("canonical root").to_string_lossy(),
        "target_revision": {"kind": "package_digest", "value": candidate},
        "command_started_at": "2026-01-01T00:00:00Z",
        "command_completed_at": "2026-01-01T00:00:01Z",
        "command_exit": 0,
        "machine_readable_report": {
            "path": "validation_artifacts/coverage/llvm-cov-full.json",
            "digest": crate::digest::file(&report).expect("report digest")
        },
        "generated_by": "coverage-command",
        "percent_source": "machine_readable_report",
        "target_paths": ["src/lib.rs"],
        "measured_dimensions": ["line"],
        "coverage": {"percent": percent, "floor_percent": 100, "policy": "100_percent_required"},
        "uncovered_records": uncovered,
        "exclusions": [],
        "generated_at": "2026-01-01T00:00:01Z",
        "claim_ceiling": if percent == 100.0 {"supports_complete_coverage_claim"} else {"withheld_or_blocked"},
        "supported_claim_classes": if percent == 100.0 {json!(["complete_coverage"])} else {json!([])},
        "blocked_claim_classes": [
            "completion", "package_readiness", "review_readiness", "release_readiness",
            "final_packet_correctness", "update_goal_eligibility",
            "app_registry_or_reviewer_exposure"
        ]
    })
}
