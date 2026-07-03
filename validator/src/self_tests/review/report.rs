use serde_json::{Value, json};
use std::path::Path;

fn write_artifact(root: &Path, rel: &str, text: &str) -> Value {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("artifact parent");
    std::fs::write(&path, text).expect("artifact write");
    json!({"path":rel,"digest":crate::digest::file(&path).expect("artifact digest")})
}

fn has_fail(failures: &[crate::review::round::ReviewFailure], error: &str) -> bool {
    failures.iter().any(|failure| failure.error == error)
}

#[test]
fn review_round_report_rejects_invalid_paths_authority_prose_and_hollow_evidence() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("review-report");
    let anchors = crate::review::round::anchor::values::fixture_anchor_values(&root);
    let receipt = json!({
        "validator_receipt":{"path":"evidence/validator.json"},
        "review_target":{"path":"evidence/review-target.json"},
        "archive":{"path":"evidence/archive.json"}
    });
    let mut failures = Vec::new();
    crate::review::round::report::report_errors(
        &root,
        &receipt,
        &json!({"review_report":{"path":"../escape.md"}}),
        &anchors,
        "product_simplicity_falsifier",
        &mut failures,
    );
    assert!(has_fail(&failures, "review_round_report_path_invalid"));

    let private_home = ["/", "Users", "/", "tree", "/", "private"].concat();
    let report = write_artifact(
        &root,
        "reports/authority.md",
        &format!("Narrative says {private_home} and claim ceiling sign off."),
    );
    failures.clear();
    crate::review::round::report::report_errors(
        &root,
        &receipt,
        &json!({"review_report":report,"evidence_artifacts_checked":[]}),
        &anchors,
        "product_simplicity_falsifier",
        &mut failures,
    );
    assert!(has_fail(&failures, "review_round_private_path_leak"));
    assert!(has_fail(&failures, "review_round_report_authority_prose"));
    assert!(has_fail(
        &failures,
        "review_round_hollow_evidence_artifacts"
    ));

    let quiet_report = write_artifact(&root, "reports/no-disclaimer.md", "quiet narrative only");
    let duplicate = write_artifact(&root, "evidence/dup.json", "{}");
    let row = json!({
        "review_report":quiet_report,
        "evidence_artifacts_checked":[duplicate, duplicate, duplicate, duplicate, duplicate, duplicate]
    });
    failures.clear();
    crate::review::round::report::report_errors(
        &root,
        &receipt,
        &row,
        &anchors,
        "product_simplicity_falsifier",
        &mut failures,
    );
    assert!(has_fail(
        &failures,
        "review_round_report_missing_disclaimer"
    ));
    assert!(has_fail(
        &failures,
        "review_round_hollow_evidence_artifacts"
    ));

    std::fs::remove_dir_all(root).expect("cleanup review report");
}

#[test]
fn review_round_report_requires_bound_artifacts_for_required_paths() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("review-report-required");
    let anchors = crate::review::round::anchor::values::fixture_anchor_values(&root);
    let disclaimer = "This fixture report is a narrative attachment only. It is not an authority for verdicts, blockers, counterexample coverage, proof anchors, claim ceilings, or next-phase routing. Those obligations live in the typed review-round receipt and are validated by Rust.";
    let report = write_artifact(&root, "reports/disclaimer.md", disclaimer);
    for rel in [
        "evidence/validator.json",
        "evidence/review-target.json",
        "evidence/archive.json",
        "agents/prompt.md",
        "agents/custom.toml",
        "extra/one.json",
    ] {
        write_artifact(&root, rel, "{}");
    }
    let receipt = json!({
        "validator_receipt":{"path":"evidence/validator.json"},
        "review_target":{"path":"evidence/review-target.json"},
        "archive":{"path":"evidence/archive.json"}
    });
    let row = json!({
        "review_report":report,
        "persona_prompt_path":"agents/prompt.md",
        "custom_agent_path":"agents/custom.toml",
        "evidence_artifacts_checked":[
            write_artifact(&root, "evidence/validator.json", "{}"),
            write_artifact(&root, "evidence/review-target.json", "{}"),
            write_artifact(&root, "evidence/archive.json", "{}"),
            write_artifact(&root, "agents/prompt.md", "{}"),
            write_artifact(&root, "agents/custom.toml", "{}"),
            write_artifact(&root, "extra/one.json", "{}")
        ]
    });
    let mut failures = Vec::new();
    crate::review::round::report::report_errors(
        &root,
        &receipt,
        &row,
        &anchors,
        "contract_claim_falsifier",
        &mut failures,
    );
    assert!(has_fail(
        &failures,
        "review_round_hollow_evidence_artifacts"
    ));
    std::fs::remove_dir_all(root).expect("cleanup review required");
}
