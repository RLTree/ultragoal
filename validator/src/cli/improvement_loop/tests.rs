use super::{ImprovementLoopCommand, proof, registry};
use serde_json::json;
use std::path::{Path, PathBuf};

#[test]
fn improvement_loop_receipt_fails_closed_for_partial_loop() {
    let root = crate::self_tests::boundaries::support::temp_root("improvement-loop-partial");
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
    assert!(
        receipt["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure == "improvement_loop_not_complete_same_candidate")
    );
}

#[test]
fn improvement_loop_receipt_has_possible_green_path() {
    let root = crate::self_tests::boundaries::support::temp_root("improvement-loop-green");
    seed_root(&root, "complete_same_candidate");
    let command = ImprovementLoopCommand {
        receipt: PathBuf::from(super::DEFAULT_RECEIPT),
    };
    let receipt = proof::build_receipt(&root, &command).expect("receipt");
    assert_eq!(receipt["status"], "pass");
    assert!(proof::receipt_failures(&root, &receipt).is_empty());
}

#[test]
fn improvement_loop_receipt_rejects_hand_authored_green() {
    let root = crate::self_tests::boundaries::support::temp_root("improvement-loop-tamper");
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

fn seed_root(root: &Path, closure_status: &str) {
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::create_dir_all(root.join("validation_artifacts/promptfoo")).expect("promptfoo");
    std::fs::create_dir_all(root.join("validation_artifacts/halo")).expect("halo");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources": [super::REGISTRY]}),
    )
    .expect("manifest");
    crate::json_boundary::write_json(&root.join(super::REGISTRY), &registry_doc(closure_status))
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

fn registry_doc(closure_status: &str) -> serde_json::Value {
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
            "loop_id": "source-local-gate-94-fixture",
            "loop_closure_status": closure_status,
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
