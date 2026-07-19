use super::*;

#[test]
pub(crate) fn evaluation_audit_binds_typed_spec_to_current_context_without_writes() {
    let repo = Repository::new("evaluation-audit");
    write_spec(&repo.root, "audit-spec", false);
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "eval", "audit", "--spec", "evaluation/audit.json"]).unwrap()
    else {
        panic!("expected evaluation audit invocation");
    };

    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);

    assert_eq!(streams.exit_code, 0);
    assert!(streams.stderr.is_empty());
    let output: serde_json::Value = serde_json::from_slice(&streams.stdout).unwrap();
    assert_eq!(output["schema_version"], "EvaluationAudit-v1");
    assert_eq!(output["audit"]["eligible"], true);
    assert!(
        output["candidate_id"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:"))
    );
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

#[test]
pub(crate) fn evaluation_audit_rejects_extra_input_without_echo_or_writes() {
    let repo = Repository::new("evaluation-audit-invalid");
    let canary = "private-evaluation-canary";
    write_spec(&repo.root, canary, true);
    let before_tree = tree(&repo.root);
    let before_status = repo.status();
    let ParseOutcome::Invocation(invocation) =
        parse_args(["--json", "eval", "audit", "--spec", "evaluation/audit.json"]).unwrap()
    else {
        panic!("expected evaluation audit invocation");
    };

    let streams = execute_invocation(&repo.root, invocation).render(OutputMode::Json);

    assert_eq!(streams.exit_code, 2);
    assert!(streams.stdout.is_empty());
    let diagnostic = String::from_utf8(streams.stderr).unwrap();
    assert!(diagnostic.contains("successor_runtime_unexpected_arguments"));
    assert!(!diagnostic.contains(canary));
    assert_eq!(tree(&repo.root), before_tree);
    assert_eq!(repo.status(), before_status);
}

fn write_spec(root: &std::path::Path, spec_id: &str, extra: bool) {
    let sha = |character| format!("sha256:{}", character.to_string().repeat(64));
    let mut value = serde_json::json!({
        "schema_version": "EvaluationAuditSpec-v1",
        "spec_id": spec_id,
        "tasks": [{
            "task_id": "audit-core",
            "requirement_id": "REQ-audit",
            "behavior_id": "audit-behavior",
            "fixture_id": "audit-fixture",
            "dataset": {"relative_path": "fixtures/audit.json", "digest_sha256": sha('a'), "byte_length": 32},
            "dataset_provenance_sha256": sha('b'),
            "known_training_corpus_sha256s": [],
            "scorer_id": "audit-scorer",
            "scorer_digest_sha256": sha('c'),
            "perturbation_controls": ["verbosity", "proof_artifact", "receipt_production", "test_manipulation", "score_only"],
            "representative": true,
            "data_controls": {
                "objective": "audit-behavior",
                "success_criterion": "audit-behavior-passes",
                "failure_criterion": "audit-behavior-fails",
                "split_id": "audit-split",
                "training_split_ids": [],
                "semantic_fingerprint_sha256": sha('d'),
                "near_duplicate_group_sha256": sha('e'),
                "known_training_fingerprint_sha256s": [],
                "declared_label": "audit-behavior",
                "verified_label": "audit-behavior",
                "sampled_population": "audit-population",
                "target_population": "audit-population"
            }
        }]
    });
    if extra {
        value["extra"] = serde_json::Value::String("private-evaluation-canary".to_owned());
    }
    std::fs::create_dir_all(root.join("evaluation")).unwrap();
    std::fs::write(
        root.join("evaluation/audit.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
}
