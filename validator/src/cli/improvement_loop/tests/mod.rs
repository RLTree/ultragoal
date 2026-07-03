use super::{ImprovementLoopCommand, proof, registry};
use serde_json::json;
use std::path::{Path, PathBuf};

mod edges;
mod stage_evidence;

#[test]
fn improvement_loop_receipt_fails_closed_for_partial_loop() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("improvement-loop-partial");
    seed_root(&root, "partial");
    let command = ImprovementLoopCommand {
        receipt: PathBuf::from(super::DEFAULT_RECEIPT),
    };
    let receipt = proof::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(
        receipt["raw_observation_authority"],
        "observation_only_until_cli_parsed_loop_receipt"
    );
    assert!(
        registry::array_strings(receipt.get("blocked_claims")).contains("update_goal_eligibility")
    );
    let failures = receipt["failures"].as_array().unwrap();
    assert!(
        failures
            .iter()
            .any(|failure| failure == "improvement_loop_not_complete_same_candidate")
    );
}

#[test]
fn improvement_loop_receipt_has_possible_green_path() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("improvement-loop-green");
    seed_root(&root, "complete_same_candidate");
    let command = ImprovementLoopCommand {
        receipt: PathBuf::from(super::DEFAULT_RECEIPT),
    };
    let receipt = proof::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert!(proof::receipt_failures(&root, &receipt).is_empty());
}

#[test]
fn improvement_loop_receipt_rejects_missing_stage_evidence() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "improvement-loop-no-evidence",
    );
    seed_root(&root, "complete_same_candidate");
    let mut registry_doc = crate::json_boundary::read_json(&root.join(super::REGISTRY)).unwrap();
    registry_doc["loops"][0]
        .as_object_mut()
        .unwrap()
        .remove("stage_evidence_path");
    crate::json_boundary::write_json(&root.join(super::REGISTRY), &registry_doc).unwrap();
    let command = ImprovementLoopCommand {
        receipt: PathBuf::from(super::DEFAULT_RECEIPT),
    };
    let receipt = proof::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "fail");
    let failures = receipt["failures"].as_array().unwrap();
    assert!(failures.iter().any(|failure| {
        failure == "improvement_loop_stage_evidence_missing_field:stage_evidence_path"
    }));
}

#[test]
fn improvement_loop_receipt_rejects_hand_authored_green() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("improvement-loop-tamper");
    seed_root(&root, "partial");
    let mut receipt = json!({
        "schema": super::RECEIPT_SCHEMA,
        "status": "pass",
        "candidate_digest": crate::digest::ZERO,
        "registry_path": super::REGISTRY,
        "registry_digest": crate::digest::ZERO,
        "loop_ids": ["loop"],
        "promptfoo_adapter_receipt_digest": crate::digest::ZERO,
        "halo_capability_receipt_digest": crate::digest::ZERO,
        "raw_observation_authority": "raw_halo_output",
        "observability_receipt": {},
        "supported_claims": ["harness_improvement_loop_closure"],
        "blocked_claims": [],
        "claim_impact": "supports_everything",
        "failures": []
    });
    receipt["candidate_digest"] = json!(crate::package::inventory::package_digest(&root).unwrap());
    assert!(!proof::receipt_failures(&root, &receipt).is_empty());
}

#[test]
fn improvement_loop_defaults_and_run_error_boundaries_are_typed() {
    let default = super::parse(&["improvement-loop".to_string(), "prove".to_string()])
        .expect("parse default")
        .expect("improvement loop default");
    assert_eq!(default.receipt, PathBuf::from(super::DEFAULT_RECEIPT));
    super::print_receipt(std::path::Path::new("receipt.json"), &json!({}));

    let missing_root = std::env::temp_dir().join(format!(
        "ultragoal-improvement-loop-missing-root-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&missing_root);
    assert!(super::run(&missing_root, &default).is_err());

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "improvement-loop-write-error",
    );
    seed_root(&root, "partial");
    std::fs::remove_dir_all(root.join("validation_artifacts")).expect("remove artifacts dir");
    std::fs::write(root.join("validation_artifacts"), "not a directory").expect("blocker");
    let command = ImprovementLoopCommand {
        receipt: PathBuf::from("validation_artifacts/improvement-loop/write-error.json"),
    };
    assert!(super::run(&root, &command).is_err());
    std::fs::remove_dir_all(root).expect("cleanup write error");
}

fn seed_root(root: &Path, closure_status: &str) {
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::create_dir_all(root.join("validation_artifacts/promptfoo")).expect("promptfoo");
    std::fs::create_dir_all(root.join("validation_artifacts/halo")).expect("halo");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources": [super::REGISTRY]}),
    )
    .expect("manifest");
    crate::json_boundary::write_json(&root.join("stage-evidence.json"), &stage_evidence_doc())
        .expect("stage evidence");
    let digest = crate::digest::file(&root.join("stage-evidence.json")).expect("stage digest");
    crate::json_boundary::write_json(
        &root.join(super::REGISTRY),
        &registry_doc(closure_status, &digest),
    )
    .expect("registry");
    std::fs::write(
        root.join("validation_artifacts/promptfoo/adapter-receipt.json"),
        "{}",
    )
    .expect("promptfoo receipt");
    std::fs::write(
        root.join("validation_artifacts/halo/capability-receipt.json"),
        "{}",
    )
    .expect("halo receipt");
}

fn registry_doc(closure_status: &str, evidence_digest: &str) -> serde_json::Value {
    json!({
        "schema": "harness-ultragoal.improvement-loop-registry.v1",
        "law_id": super::LAW_ID,
        "forbidden_substitutions": [
            "raw_trace_as_loop_closure",
            "raw_feedback_as_loop_closure",
            "raw_model_output_as_loop_closure",
            "raw_promptfoo_output_as_loop_closure",
            "raw_halo_output_as_loop_closure",
            "reviewer_agreement_as_loop_closure",
            "checklist_prose_as_loop_closure",
            "hand_authored_receipt_as_loop_closure"
        ],
        "loops": [{
            "loop_id": "source-local-agent-improvement-loop-fixture",
            "loop_closure_status": closure_status,
            "stage_evidence_path": "stage-evidence.json",
            "stage_evidence_schema": "harness-ultragoal.improvement-loop-stage-evidence.v1",
            "stage_evidence_digest": evidence_digest,
            "trace_ids": ["trace"],
            "feedback_ids": ["feedback"],
            "cluster_ids": ["cluster"],
            "eval_ids": ["eval"],
            "promptfoo_suite_ids": ["suite"],
            "halo_ranking_ids": ["ranking"],
            "codex_handoff_ids": ["handoff"],
            "implementation_change_ids": ["change"],
            "validation_receipt_ids": ["validation"],
            "before_after_telemetry_ids": ["comparison"],
            "promotion_ids": ["promotion"],
            "stage_statuses": registry::REQUIRED_STAGES
                .iter()
                .map(|stage| json!({"stage_id": stage, "status": "complete"}))
                .collect::<Vec<_>>()
        }]
    })
}

fn stage_evidence_doc() -> serde_json::Value {
    json!({
        "schema": "harness-ultragoal.improvement-loop-stage-evidence.v1",
        "law_id": super::LAW_ID,
        "loop_id": "source-local-agent-improvement-loop-fixture",
        "status": "pass",
        "authority": "cli_parsed_package_static_stage_evidence",
        "candidate_binding": "package_static_source_evidence",
        "claim_impact": "stage_only",
        "stages": registry::REQUIRED_STAGES
            .iter()
            .map(|stage| json!({
                "stage_id": stage,
                "evidence_id": format!("evidence-{stage}"),
                "evidence_kind": "fixture",
                "status": "pass",
                "source_paths": ["plugin-manifest-draft.json"],
                "receipt_paths": ["validation_artifacts/promptfoo/adapter-receipt.json"],
                "command_ids": ["fixture-command"],
                "forbidden_substitutions_rejected": ["fixture-substitution"],
                "claim_impact": "stage_only"
            }))
            .collect::<Vec<_>>()
    })
}
