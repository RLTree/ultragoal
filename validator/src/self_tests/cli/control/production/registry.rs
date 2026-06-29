use crate::cli::control::plane::types::ControlOperation;
use crate::cli::control::plane::{ControlCommand, receipt, run};
use serde_json::json;
use std::path::Path;

fn write_json(path: &Path, value: &serde_json::Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
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

fn write_registry_probe_root(root: &Path) {
    let repo = crate::self_tests::boundaries::support::repo_root();
    copy_flat_dir(root, &repo, "schemas");
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"name":"harness-ultragoal","version":"0.0.0-test"}),
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({
            "version":"0.0.0-test",
            "schema_catalog":"schemas/schema-catalog.json",
            "schemas":["schemas/codex-registry-exposure.schema.json"],
            "resources":["schemas/codex-registry-exposure.schema.json"]
        }),
    );
}

#[test]
fn registry_probe_reports_registry_surface_without_packet_circularity() {
    let root = crate::self_tests::boundaries::support::temp_root("cli-registry-probe-specific");
    write_registry_probe_root(&root);
    let command = ControlCommand {
        operation: ControlOperation::RegistryProbe,
        receipt: None,
        surface_root: None,
    };
    let value = receipt(&root, &command).expect("receipt");
    assert_eq!(value["status"], "fail");
    assert_eq!(
        value["required_evidence"],
        json!(["live_registry_or_reviewer_exposure_same_surface_pass"])
    );
    assert!(
        value["blocked_claim_classes"]
            .as_array()
            .expect("blocked claims")
            .iter()
            .any(|claim| claim.as_str() == Some("app_registry_or_reviewer_exposure"))
    );
    let observed = value["failure"]["observed_value"]
        .as_str()
        .expect("observed");
    assert!(observed.contains("registry_surface:plugin_self_law_json_missing_or_malformed"));
    assert!(!observed.contains("final_packet"), "{observed}");

    let receipt_path = root.join("validation_artifacts/cli/registry-probe-receipt.json");
    let exit = run(
        &root,
        &ControlCommand {
            operation: ControlOperation::RegistryProbe,
            receipt: Some(receipt_path.clone()),
            surface_root: None,
        },
    )
    .expect("registry probe command");
    assert_eq!(exit, 1);
    let active_path =
        root.join("validation_artifacts/ultragoal-audit/active-registry-exposure-current.json");
    let active = crate::json_boundary::read_json(&active_path).expect("active registry receipt");
    assert_eq!(active["status"], "fail");
    assert_eq!(active["claim_ceiling"], "withheld_or_blocked");
    assert_eq!(active["source"], "ultragoal.registry_probe");
    assert_eq!(
        active["target_revision"]["value"],
        crate::package::inventory::package_digest(&root).expect("digest")
    );
    let store = crate::schema_catalog::load(&root);
    assert!(
        crate::schema_catalog::schema_errors(
            &store,
            "codex-registry-exposure.schema.json",
            &active
        )
        .is_empty(),
        "{active}"
    );
    let command_receipt =
        crate::json_boundary::read_json(&receipt_path).expect("registry command receipt");
    assert_eq!(command_receipt["status"], "fail");
    assert!(
        command_receipt["failure"]["observed_value"]
            .as_str()
            .expect("observed after mint")
            .contains("plugin_self_law_registry_status_not_pass")
    );
    std::fs::remove_dir_all(root).expect("cleanup registry probe");
}
