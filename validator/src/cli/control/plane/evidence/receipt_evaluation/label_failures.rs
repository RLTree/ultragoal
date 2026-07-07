use serde_json::json;

fn blocked_claims() -> serde_json::Value {
    json!([
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "final_packet_correctness",
        "update_goal_eligibility",
        "app_registry_or_reviewer_exposure"
    ])
}

#[test]
fn label_failures_cover_unknown_rust_and_gc_status_edges() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('d');
    assert_eq!(
        super::label_failures(&root, "unknown", &json!({}), &candidate),
        vec!["unknown_evidence_label:unknown"]
    );
    let rust = json!({
        "schema":"harness-ultragoal.rust-devx-receipt.v1",
        "law_id":"rust-command-loop-authority",
        "status":"fail",
        "digests":{"candidate":candidate},
        "observations":[],
        "observation_failures":[],
        "claim_ceiling":"rust_devx_observation_bound"
    });
    assert!(
        super::label_failures(&root, "rust_fast", &rust, &candidate)
            .iter()
            .any(|failure| failure == "rust_receipt_status_not_pass")
    );
    for (label, law_id) in [
        ("rust_dependency", "rust-developer-experience-authority"),
        (
            "rust_workspace_topology",
            "rust-developer-experience-authority",
        ),
        ("rust_toolchain", "rust-toolchain-substrate-authority"),
        ("rust_clean_proof", "rust-cache-no-cache-honesty"),
        ("rust_memory", "rust-memory-resource-discipline"),
    ] {
        let receipt = json!({
            "schema":"harness-ultragoal.rust-devx-receipt.v1",
            "law_id":law_id,
            "status":"fail",
            "digests":{"candidate":candidate},
            "observations":[],
            "observation_failures":[],
            "claim_ceiling":"rust_devx_observation_bound"
        });
        assert!(
            super::label_failures(&root, label, &receipt, &candidate)
                .iter()
                .any(|failure| failure == "rust_receipt_status_not_pass"),
            "{label}"
        );
    }
    let source_audit = json!({
        "status":"fail",
        "target_revision":{"kind":"package_digest","value":candidate}
    });
    assert!(
        super::label_failures(&root, "source_audit", &source_audit, &candidate)
            .iter()
            .any(|failure| failure == "source_audit_status_not_pass")
    );
    let source_audit_pass = json!({
        "status":"pass",
        "target_revision":{"kind":"package_digest","value":candidate},
        "claim_ceiling":"source_audit_pass_source_local_only",
        "supported_claim_classes":["source_local_audit_checks", "red_fixture_report"],
        "blocked_claim_classes": blocked_claims()
    });
    assert!(
        super::label_failures(&root, "source_audit", &source_audit_pass, &candidate).is_empty()
    );
    let source_audit_wrong_target = json!({
        "status":"pass",
        "target_revision":{"kind":"package_digest","value":crate::self_tests::boundaries::workspace_fixtures::sha('c')},
        "claim_ceiling":"source_audit_pass_source_local_only",
        "supported_claim_classes":["source_local_audit_checks", "red_fixture_report"],
        "blocked_claim_classes": blocked_claims()
    });
    assert!(
        super::label_failures(
            &root,
            "source_audit",
            &source_audit_wrong_target,
            &candidate
        )
        .iter()
        .any(|failure| failure == "source_audit_target_digest_mismatch")
    );
    let coverage_wrong_target = json!({
        "coverage_target_dir":"target/ultragoal-coverage",
        "target_revision":{"kind":"package_digest","value":crate::self_tests::boundaries::workspace_fixtures::sha('e')},
        "coverage":{"percent":100.0},
        "uncovered_records":[],
        "claim_ceiling":"supports_complete_coverage_claim",
        "supported_claim_classes":["complete_coverage"],
        "blocked_claim_classes": blocked_claims()
    });
    assert!(
        super::label_failures(&root, "coverage", &coverage_wrong_target, &candidate)
            .iter()
            .any(|failure| failure == "coverage_target_digest_mismatch")
    );
    let coverage_pass = json!({
        "coverage_target_dir":"target/ultragoal-coverage",
        "target_revision":{"kind":"package_digest","value":candidate},
        "coverage":{"percent":100.0},
        "uncovered_records":[],
        "claim_ceiling":"supports_complete_coverage_claim",
        "supported_claim_classes":["complete_coverage"],
        "blocked_claim_classes": blocked_claims()
    });
    assert!(super::label_failures(&root, "coverage", &coverage_pass, &candidate).is_empty());
    let coverage_missing_target_dir = json!({
        "target_revision":{"kind":"package_digest","value":candidate},
        "coverage":{"percent":100.0},
        "uncovered_records":[],
        "claim_ceiling":"supports_complete_coverage_claim",
        "supported_claim_classes":["complete_coverage"],
        "blocked_claim_classes": blocked_claims()
    });
    assert!(
        super::label_failures(&root, "coverage", &coverage_missing_target_dir, &candidate)
            .iter()
            .any(|failure| failure == "coverage_target_dir_missing")
    );
    let coverage_not_exact = json!({
        "coverage_target_dir":"target/ultragoal-coverage",
        "target_revision":{"kind":"package_digest","value":candidate},
        "coverage":{"percent":99.0},
        "uncovered_records":["validator/src/main.rs:1"],
        "claim_ceiling":"supports_complete_coverage_claim",
        "supported_claim_classes":["complete_coverage"],
        "blocked_claim_classes": blocked_claims()
    });
    assert!(
        super::label_failures(&root, "coverage", &coverage_not_exact, &candidate)
            .iter()
            .any(|failure| failure == "coverage_not_exact_100")
    );
    let coverage_without_uncovered_records = json!({
        "coverage_target_dir":"target/ultragoal-coverage",
        "target_revision":{"kind":"package_digest","value":candidate},
        "coverage":{"percent":100.0},
        "claim_ceiling":"supports_complete_coverage_claim",
        "supported_claim_classes":["complete_coverage"],
        "blocked_claim_classes": blocked_claims()
    });
    assert!(
        super::label_failures(
            &root,
            "coverage",
            &coverage_without_uncovered_records,
            &candidate
        )
        .iter()
        .any(|failure| failure == "coverage_not_exact_100")
    );
    let coverage_with_uncovered_records = json!({
        "coverage_target_dir":"target/ultragoal-coverage",
        "target_revision":{"kind":"package_digest","value":candidate},
        "coverage":{"percent":100.0},
        "uncovered_records":["validator/src/main.rs:1"],
        "claim_ceiling":"supports_complete_coverage_claim",
        "supported_claim_classes":["complete_coverage"],
        "blocked_claim_classes": blocked_claims()
    });
    assert!(
        super::label_failures(
            &root,
            "coverage",
            &coverage_with_uncovered_records,
            &candidate
        )
        .iter()
        .any(|failure| failure == "coverage_not_exact_100")
    );
    let coverage_wrong_claim_ceiling = json!({
        "coverage_target_dir":"target/ultragoal-coverage",
        "target_revision":{"kind":"package_digest","value":candidate},
        "coverage":{"percent":100.0},
        "uncovered_records":[],
        "claim_ceiling":"withheld_or_blocked"
    });
    assert!(
        super::label_failures(&root, "coverage", &coverage_wrong_claim_ceiling, &candidate)
            .iter()
            .any(|failure| failure == "coverage_claim_ceiling_not_complete")
    );
    let gc = json!({
        "schema":"harness-ultragoal.workspace-gc-receipt.v1",
        "law_id":"workspace-artifact-cache-garbage-collection",
        "status":"fail",
        "digests":{"candidate":candidate},
        "plan":{"id":"plan"},
        "observations":[],
        "observation_failures":[],
        "claim_ceiling":"gc_observation_bound"
    });
    assert!(
        super::label_failures(&root, "gc_plan", &gc, &candidate)
            .iter()
            .any(|failure| failure == "gc_receipt_status_not_pass")
    );
    let rust_wrong_candidate = json!({
        "schema":"harness-ultragoal.rust-devx-receipt.v1",
        "law_id":"rust-command-loop-authority",
        "status":"pass",
        "digests":{"candidate":crate::self_tests::boundaries::workspace_fixtures::sha('e')},
        "observations":[],
        "observation_failures":[],
        "claim_ceiling":"rust_devx_observation_bound"
    });
    assert!(
        super::label_failures(&root, "rust_fast", &rust_wrong_candidate, &candidate)
            .iter()
            .any(|failure| failure == "rust_receipt_candidate_digest_mismatch")
    );
    let gc_wrong_candidate = json!({
        "schema":"harness-ultragoal.workspace-gc-receipt.v1",
        "law_id":"workspace-artifact-cache-garbage-collection",
        "status":"pass",
        "digests":{"candidate":crate::self_tests::boundaries::workspace_fixtures::sha('e')},
        "plan":{"id":"plan"},
        "observations":[],
        "observation_failures":[],
        "claim_ceiling":"gc_observation_bound"
    });
    assert!(
        super::label_failures(&root, "gc_plan", &gc_wrong_candidate, &candidate)
            .iter()
            .any(|failure| failure == "gc_receipt_candidate_digest_mismatch")
    );
}
