use super::seed_root;
use crate::cli::improvement_loop::{ImprovementLoopCommand, proof};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn improvement_loop_stage_evidence_failures_are_actionable() {
    assert_stage_failure(
        "stage-path-invalid",
        |root, doc| {
            doc["loops"][0]["stage_evidence_path"] = json!("../outside.json");
            crate::json_boundary::write_json(&root.join(super::super::REGISTRY), doc).unwrap();
        },
        "improvement_loop_stage_evidence_path_invalid",
    );
    assert_stage_failure(
        "stage-missing-file",
        |root, doc| {
            doc["loops"][0]["stage_evidence_path"] = json!("missing-stage.json");
            doc["loops"][0]["stage_evidence_digest"] = json!(crate::digest::ZERO);
            crate::json_boundary::write_json(&root.join(super::super::REGISTRY), doc).unwrap();
        },
        "improvement_loop_stage_evidence_missing_file",
    );
    assert_stage_failure(
        "stage-digest-mismatch",
        |root, doc| {
            let stage = root.join("stage.json");
            crate::json_boundary::write_json(
                &stage,
                &json!({
                    "schema": "harness-ultragoal.improvement-loop-stage-evidence.v1",
                    "law_id": super::super::LAW_ID,
                    "status": "pass",
                    "authority": "cli_parsed_package_static_stage_evidence",
                    "candidate_binding": "package_static_source_evidence",
                    "loop_id": "complete_same_candidate",
                    "stages": []
                }),
            )
            .unwrap();
            doc["loops"][0]["stage_evidence_path"] = json!("stage.json");
            doc["loops"][0]["stage_evidence_digest"] = json!(crate::digest::ZERO);
            crate::json_boundary::write_json(&root.join(super::super::REGISTRY), doc).unwrap();
        },
        "improvement_loop_stage_evidence_digest_mismatch",
    );
    assert_stage_failure(
        "stage-malformed-json",
        |root, doc| {
            std::fs::write(root.join("bad-stage.json"), "{").unwrap();
            doc["loops"][0]["stage_evidence_path"] = json!("bad-stage.json");
            doc["loops"][0]["stage_evidence_digest"] =
                json!(crate::digest::file(&root.join("bad-stage.json")).unwrap());
            crate::json_boundary::write_json(&root.join(super::super::REGISTRY), doc).unwrap();
        },
        "improvement_loop_stage_evidence_json_missing_or_malformed:",
    );
    assert_stage_failure(
        "stage-bad-doc",
        |root, doc| {
            let bad = json!({
                "schema": "wrong",
                "law_id": "wrong",
                "status": "fail",
                "authority": "raw",
                "candidate_binding": "raw",
                "loop_id": "wrong",
                "stages": [{
                    "stage_id": "trace_capture",
                    "status": "fail",
                    "source_paths": ["../bad"],
                    "receipt_paths": [],
                    "command_ids": [],
                    "forbidden_substitutions_rejected": []
                }]
            });
            crate::json_boundary::write_json(&root.join("bad-stage.json"), &bad).unwrap();
            doc["loops"][0]["stage_evidence_path"] = json!("bad-stage.json");
            doc["loops"][0]["stage_evidence_digest"] =
                json!(crate::digest::file(&root.join("bad-stage.json")).unwrap());
            crate::json_boundary::write_json(&root.join(super::super::REGISTRY), doc).unwrap();
        },
        "improvement_loop_stage_evidence_field_mismatch:schema",
    );
}

fn assert_stage_failure(
    label: &str,
    mutate: impl FnOnce(&std::path::Path, &mut serde_json::Value),
    expected_prefix: &str,
) {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    seed_root(&root, "complete_same_candidate");
    let mut doc = crate::json_boundary::read_json(&root.join(super::super::REGISTRY)).unwrap();
    mutate(&root, &mut doc);
    let command = ImprovementLoopCommand {
        receipt: PathBuf::from(super::super::DEFAULT_RECEIPT),
    };
    let receipt = proof::build_receipt(&root, &command).expect("receipt");
    let failures = receipt["failures"].as_array().expect("failures");
    assert!(
        failures.iter().any(|failure| failure
            .as_str()
            .is_some_and(|text| text.starts_with(expected_prefix))),
        "{expected_prefix}: {failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
