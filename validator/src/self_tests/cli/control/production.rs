use crate::cli::control::plane::types::ControlOperation;
use crate::cli::control::plane::{ControlCommand, receipt};
use serde_json::json;
use std::path::Path;

mod receipts;
mod registry;

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, text).expect("write text");
}

fn copy_file(root: &Path, repo: &Path, rel: &str) {
    if let Some(parent) = root.join(rel).parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::copy(repo.join(rel), root.join(rel)).expect("copy file");
}

fn copy_flat_dir(root: &Path, repo: &Path, rel: &str) {
    let src = repo.join(rel);
    let dst = root.join(rel);
    std::fs::create_dir_all(&dst).expect("copy dir root");
    for entry in std::fs::read_dir(src).expect("read dir") {
        let entry = entry.expect("entry");
        let child = entry.path();
        let child_rel = child.strip_prefix(repo).expect("strip repo");
        let child_rel = child_rel.to_string_lossy().replace('\\', "/");
        if child.is_file() {
            copy_file(root, repo, &child_rel);
        }
    }
}

fn write_control_green_root(root: &Path) -> String {
    let repo = crate::self_tests::boundaries::support::repo_root();
    copy_flat_dir(root, &repo, "schemas");
    for rel in [
        "templates/RED_FIXTURES.json",
        "templates/agent-standards/enforcement.json",
        "docs/source-obligation-matrix.json",
        "docs/foundational-law-traceability.json",
        "fixtures/mandatory-law-surfaces/valid/cli-performance-latency-speed-iteration-fitness.json",
    ] {
        copy_file(root, &repo, rel);
    }
    write_text(
        root.join("validator/src/cli/performance.rs").as_path(),
        "source\n",
    );
    write_text(
        root.join("validator/src/cli/performance/types.rs")
            .as_path(),
        "types\n",
    );
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"name":"harness-ultragoal","version":"0.0.0-test"}),
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "version":"0.0.0-test",
            "schema_catalog":"schemas/schema-catalog.json",
            "schemas":["schemas/cli-performance-receipt.schema.json"],
            "resources":[
                "validator/src/cli/performance.rs",
                "validator/src/cli/performance/types.rs",
                "schemas/cli-performance-receipt.schema.json",
                "validation_artifacts/cli/performance-receipt.json"
            ]
        }),
    );
    let current = crate::package::inventory::package_digest(root).expect("digest");
    for (operation, rel) in [
        (
            ControlOperation::InstallAudit,
            "validation_artifacts/cli/install-audit-receipt.json",
        ),
        (
            ControlOperation::CacheAudit,
            "validation_artifacts/cli/cache-audit-receipt.json",
        ),
    ] {
        let command = ControlCommand {
            operation,
            receipt: None,
            surface_root: Some(root.to_path_buf()),
        };
        let receipt =
            crate::cli::control::plane::surface::receipt(root, &command).expect("surface receipt");
        write_json(&root.join(rel), &receipt);
    }
    crate::self_tests::audit::final_packet::support::write_green_proof(root, &current);
    receipts::write_standards_rust_gc(root, &current);
    write_json(
        &root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json"),
        &json!({
            "schema":"harness-ultragoal.red-fixture-report.v1",
            "status":"pass",
            "generated_at":"2026-06-27T00:00:00Z",
            "target_revision":{"kind":"package_digest","value":current},
            "red_fixtures":{
                "row":{"status":"pass","packet_path":"fixtures/red/row.json","packet_digest":crate::digest::ZERO}
            }
        }),
    );
    current
}

#[test]
fn production_control_plane_stays_transition_only_without_transactional_finalization() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-production-green");
    let current = write_control_green_root(&root);
    let command = ControlCommand {
        operation: ControlOperation::UpdateGoalEligibility,
        receipt: None,
        surface_root: None,
    };
    let value = receipt(&root, &command).expect("receipt");
    assert_eq!(value["status"], "fail");
    assert_eq!(value["candidate_digest"], current);
    assert_eq!(value["issuer"]["self_law_state"], "transition_only");
    assert_eq!(value["claim_ceiling"], "withheld_or_blocked");
    let observed = value["failure"]["observed_value"]
        .as_str()
        .expect("observed");
    assert!(
        observed.contains("cli_control_plane_transactional_finalization_missing"),
        "{observed}"
    );
    let transaction =
        crate::cli::control::plane::transactional::receipt(&root).expect("transaction receipt");
    assert_eq!(transaction["status"], "pass", "{transaction}");
    write_json(
        &root.join("validation_artifacts/cli/transactional-finalization-receipt.json"),
        &transaction,
    );
    let green = receipt(&root, &command).expect("green receipt");
    assert_eq!(green["status"], "pass", "{green}");
    assert_eq!(green["issuer"]["self_law_state"], "self_hosted");
    assert_eq!(green["claim_ceiling"], "supports_update_goal_eligibility");
    assert_eq!(
        green["evidence_graph"]["evaluation_mode"],
        "production_dereferenced"
    );

    std::fs::write(
        root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json"),
        serde_json::to_vec(&json!({
            "schema":"harness-ultragoal.red-fixture-report.v1",
            "status":"fail",
            "generated_at":"2026-06-27T00:00:00Z",
            "red_fixtures":{
                "row":{"status":"pass","packet_path":"fixtures/red/row.json","packet_digest":crate::digest::ZERO}
            }
        }))
        .expect("red json"),
    )
    .expect("write red report");
    let blocked = receipt(&root, &command).expect("blocked receipt");
    assert_eq!(blocked["status"], "fail");
    assert!(
        blocked["failure"]["observed_value"]
            .as_str()
            .expect("observed")
            .contains("red_fixture_report_status_not_pass")
    );
    std::fs::remove_dir_all(root).expect("cleanup cli production green");
}
