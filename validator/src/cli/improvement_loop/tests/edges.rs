use super::seed_root;
use crate::cli::improvement_loop::{ImprovementLoopCommand, proof, registry};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn improvement_loop_run_parse_and_receipt_tamper_edges_are_typed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("improvement-loop-run");
    seed_root(&root, "complete_same_candidate");
    let receipt_rel = "validation_artifacts/improvement-loop/run-receipt.json";
    let receipt_path = root.join(receipt_rel);
    assert!(
        crate::cli::improvement_loop::parse(&["not-improvement-loop".to_string()])
            .expect("non improvement loop")
            .is_none()
    );
    assert!(crate::cli::improvement_loop::parse(&["improvement-loop".to_string()]).is_err());
    let command = parse_loop(&["improvement-loop", "prove", "--receipt", receipt_rel]);
    assert_eq!(
        crate::cli::improvement_loop::run(&root, &command).expect("run improvement loop"),
        0
    );
    let receipt = crate::json_boundary::read_json(&receipt_path).expect("receipt");
    assert!(crate::cli::improvement_loop::receipt_failures(&root, &receipt).is_empty());

    let relative = parse_loop(&[
        "improvement-loop",
        "prove",
        "--receipt",
        "validation_artifacts/improvement-loop/relative-receipt.json",
    ]);
    assert_eq!(
        crate::cli::improvement_loop::run(&root, &relative).expect("relative run"),
        0
    );
    assert!(
        root.join("validation_artifacts/improvement-loop/relative-receipt.json")
            .is_file()
    );

    let tampered = json!({
        "schema": "wrong",
        "status": "pass",
        "candidate_digest": crate::digest::ZERO,
        "registry_digest": crate::digest::ZERO,
        "raw_observation_authority": "raw_trace",
        "observability_receipt": {},
        "blocked_claims": []
    });
    let failures = crate::cli::improvement_loop::receipt_failures(&root, &tampered);
    for expected in [
        "improvement_loop_field_mismatch:schema",
        "improvement_loop_field_mismatch:candidate_digest",
        "improvement_loop_field_mismatch:raw_observation_authority",
        "improvement_loop_receipt_digest_mismatch:registry_digest",
        "improvement_loop_receipt_missing_claim_blockers",
        "improvement_loop_receipt_observability_binding_invalid",
    ] {
        assert!(
            failures.contains(&expected.to_string()),
            "{expected}: {failures:?}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn improvement_loop_run_rejects_absolute_receipt_claim_artifact() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("improvement-loop-absolute");
    seed_root(&root, "complete_same_candidate");
    let command = ImprovementLoopCommand {
        receipt: root.join("absolute-improvement-loop-receipt.json"),
    };
    let error =
        crate::cli::improvement_loop::run(&root, &command).expect_err("absolute receipt rejected");
    assert!(error.contains("improvement loop receipt"), "{error}");
    assert!(
        error.contains("root-relative claim artifact path"),
        "{error}"
    );
    assert!(error.contains("external debug only"), "{error}");
    std::fs::remove_dir_all(root).expect("cleanup absolute receipt");
}

#[test]
fn improvement_loop_registry_failures_cover_empty_and_partial_rows() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("improvement-loop-registry");
    seed_root(&root, "complete_same_candidate");
    let empty_failures = registry::registry_failures(&root, &json!({}));
    assert!(empty_failures.contains(&"improvement_loop_field_mismatch:schema".to_string()));
    assert!(empty_failures.contains(&"improvement_loop_registry_has_no_loops".to_string()));
    assert!(
        empty_failures
            .iter()
            .any(|failure| failure.starts_with("improvement_loop_forbidden_substitution_missing:"))
    );
    assert!(registry::loop_ids(&json!({})).is_empty());
    assert!(!registry::complete_same_candidate(&json!({})));

    let mut doc = crate::json_boundary::read_json(&root.join(super::super::REGISTRY)).unwrap();
    let loop_item = doc["loops"][0].as_object_mut().unwrap();
    loop_item.insert("loop_id".to_string(), json!(""));
    loop_item.insert("trace_ids".to_string(), json!([]));
    loop_item.insert(
        "stage_statuses".to_string(),
        json!([{"stage_id": "trace_capture", "status": "pending"}]),
    );
    loop_item.insert("loop_closure_status".to_string(), json!("partial"));
    let failures = registry::registry_failures(&root, &doc);
    for expected in [
        "improvement_loop_missing_loop_id",
        "improvement_loop_missing_binding:trace_ids",
        "improvement_loop_missing_stage:trace_capture",
        "improvement_loop_not_complete_same_candidate",
    ] {
        assert!(
            failures.contains(&expected.to_string()),
            "{expected}: {failures:?}"
        );
    }
    assert!(!registry::complete_same_candidate(&doc));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn improvement_loop_missing_json_dependency_is_explicit() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "improvement-loop-missing-json",
    );
    super::seed_root(&root, "complete_same_candidate");
    std::fs::write(root.join(super::super::REGISTRY), "{").expect("malformed registry");
    let command = ImprovementLoopCommand {
        receipt: PathBuf::from(super::super::DEFAULT_RECEIPT),
    };
    let receipt = proof::build_receipt(&root, &command).expect("receipt");
    assert!(
        receipt["failures"]
            .as_array()
            .expect("failures")
            .iter()
            .any(|failure| failure.as_str().is_some_and(
                |text| text.starts_with("improvement_loop_json_missing_or_malformed:")
            )),
        "{receipt}"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn parse_loop(args: &[&str]) -> ImprovementLoopCommand {
    crate::cli::improvement_loop::parse(
        &args
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>(),
    )
    .expect("parse improvement loop")
    .expect("improvement loop command")
}
