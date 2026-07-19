use super::*;
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;

#[test]
fn evaluation_run_refuses_before_custody_or_output_on_an_unsupported_host() {
    let repo = Repository::new("evaluation-run");
    write_production_inputs(&repo.root);
    let invocation = parse_run();
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let first = execute_invocation(&repo.root, invocation.clone()).render(OutputMode::Json);
    assert_eq!(
        first.exit_code,
        4,
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let replay = execute_invocation(&repo.root, invocation).render(OutputMode::Json);
    assert_eq!(
        replay.exit_code,
        4,
        "{}",
        String::from_utf8_lossy(&replay.stderr)
    );
    assert!(first.stdout.is_empty());
    assert!(replay.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&first.stderr)
            .contains("successor_runtime_downstream_tool_unavailable")
    );
    assert!(!repo.root.join("results/run.json").exists());
    assert!(!repo.root.join(".ultragoal/evaluation").exists());
    assert!(!repo.root.join(".ultragoal-evaluation-runs").exists());
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

fn parse_run() -> ParsedInvocation {
    let ParseOutcome::Invocation(invocation) = parse_args([
        "--json",
        "eval",
        "run",
        "--spec",
        "evaluation/run.json",
        "--output",
        "results/run.json",
    ])
    .unwrap() else {
        panic!("expected evaluation run invocation");
    };
    invocation
}

fn write_production_inputs(root: &std::path::Path) {
    let digest = |bytes: &[u8]| format!("sha256:{:x}", Sha256::digest(bytes));
    let sha = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
    let dataset = b"current evaluation dataset";
    let fixture = root.join("evaluation/fixtures/run-core/run.sh");
    std::fs::create_dir_all(fixture.parent().unwrap()).unwrap();
    std::fs::create_dir_all(root.join("datasets")).unwrap();
    std::fs::create_dir_all(root.join("scorers")).unwrap();
    std::fs::create_dir_all(root.join("graders")).unwrap();
    std::fs::create_dir_all(root.join("results")).unwrap();
    let script = b"#!/bin/sh\nprintf accepted > file/result.json\n";
    std::fs::write(&fixture, script).unwrap();
    std::fs::set_permissions(&fixture, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(root.join("datasets/run.json"), dataset).unwrap();
    let dataset_digest = digest(dataset);
    let scorer = serde_json::json!({
        "schema_version": "EvaluationScorer-v2", "dataset_digest_sha256": dataset_digest,
        "executable_digest_sha256": digest(script), "primary_score_possible": 1,
        "work_units": 1, "failure_causal_code": "fixture-failed",
        "perturbation_controls": ["verbosity", "proof_artifact", "receipt_production", "test_manipulation", "score_only"],
        "task_authority_id": "evaluation-authority", "task_author_principal_id": "evaluation-author",
        "task_author_session_id": sha('1')
    });
    let scorer = serde_json::to_vec(&scorer).unwrap();
    std::fs::write(root.join("scorers/run-scorer"), &scorer).unwrap();
    std::fs::write(root.join("graders/run-core.json"), serde_json::to_vec(&serde_json::json!({
        "schema_version": "IndependentArtifactTextGrader-v2", "dataset_digest_sha256": dataset_digest,
        "scorer_digest_sha256": digest(&scorer), "root_authority_id": "evaluation-grader-authority",
        "independent_grader_principal_id": "evaluation-grader", "independent_grader_session_id": sha('3'),
        "expected_artifact_text": "accepted", "score_possible": 1
    })).unwrap()).unwrap();
    std::fs::write(root.join("evaluation/run.json"), serde_json::to_vec(&serde_json::json!({
        "schema_version": "EvaluationAuditSpec-v1", "spec_id": "run-spec", "tasks": [{
            "task_id": "run-core", "requirement_id": "REQ-run", "behavior_id": "run-behavior", "fixture_id": "run-core",
            "dataset": {"relative_path": "datasets/run.json", "digest_sha256": dataset_digest, "byte_length": dataset.len()},
            "dataset_provenance_sha256": sha('4'), "known_training_corpus_sha256s": [],
            "scorer_id": "run-scorer", "scorer_digest_sha256": digest(&scorer),
            "perturbation_controls": ["verbosity", "proof_artifact", "receipt_production", "test_manipulation", "score_only"], "representative": true,
            "data_controls": {"objective":"run","success_criterion":"pass","failure_criterion":"fail","split_id":"run-split","training_split_ids":[],"semantic_fingerprint_sha256":sha('5'),"near_duplicate_group_sha256":sha('6'),"known_training_fingerprint_sha256s":[],"declared_label":"run","verified_label":"run","sampled_population":"run","target_population":"run"}
        }]
    })).unwrap()).unwrap();
}
