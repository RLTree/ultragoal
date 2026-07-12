use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn manifest(root: &Path, resources: &[&str]) {
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":resources}),
    );
}

fn gate(decision: &str) -> Value {
    json!({
        "schema": "harness-ultragoal.review-materiality-gate.v2",
        "decision_authority": "deterministic_rust",
        "reviewer_output_authority": "falsification_only_cannot_raise_claims",
        "decision": decision,
        "deterministic_gates_required": [
            "anchor_existence_and_digest",
            "validator_receipt_status",
            "reviewer_registry_role_exposure",
            "stale_receipt_check",
            "package_private_artifact_hygiene",
            "readiness_validator",
            "claim_ceiling_check",
            "proof_surface_substitution_check"
        ],
        "deterministic_gates_run": [
            "anchor_existence_and_digest",
            "validator_receipt_status",
            "reviewer_registry_role_exposure",
            "stale_receipt_check",
            "package_private_artifact_hygiene",
            "readiness_validator",
            "claim_ceiling_check",
            "proof_surface_substitution_check"
        ],
        "claim_ceiling": {"supported": [], "unsupported": [], "blocked": []}
    })
}

#[test]
fn archive_receipts_and_hygiene_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("archive-branches");
    manifest(&root, &["docs/file.txt"]);
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/file.txt"), "ok").expect("file");
    let receipt = crate::archive::build_archive(
        &root,
        &root.join("candidate.zip"),
        "harness-ultragoal",
        "candidate_review_anchor",
    )
    .expect("archive");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(
        receipt["claim_ceiling"],
        "detached candidate review anchor only; not upload or distribution proof"
    );
    assert!(
        crate::archive::build_archive(&root, &root.join("bad.zip"), "ziproot", "distribution")
            .expect_err("bad purpose")
            .contains("archive purpose must be candidate_review_anchor")
    );

    manifest(&root, &["../escape.txt"]);
    assert!(
        crate::archive::build_archive(
            &root,
            &root.join("escape.zip"),
            "harness-ultragoal",
            "candidate_review_anchor"
        )
        .expect_err("escape rejected")
        .contains("package path escapes package root")
    );
    manifest(&root, &["__pycache__/x.pyc"]);
    assert!(
        crate::archive::build_archive(
            &root,
            &root.join("junk.zip"),
            "harness-ultragoal",
            "candidate_review_anchor"
        )
        .expect_err("junk rejected")
        .contains("junk directory is not archiveable")
    );
    manifest(&root, &["docs/private.txt"]);
    let private_home = ["/", "Users", "/", "example", "/", "private"].concat();
    std::fs::write(root.join("docs/private.txt"), private_home).expect("private");
    assert!(
        crate::archive::build_archive(
            &root,
            &root.join("private.zip"),
            "harness-ultragoal",
            "candidate_review_anchor"
        )
        .expect_err("private path rejected")
        .contains("archive entry contains private home path")
    );
    std::fs::remove_dir_all(root).expect("cleanup archive");
}

#[test]
fn archive_hygiene_rejects_unowned_filesystem_edges() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("archive-filesystem-edges");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/file.txt"), "ok").expect("file");
    manifest(&root, &["docs/binary.bin"]);
    std::fs::write(root.join("docs/binary.bin"), [0xff, 0xfe, 0xfd]).expect("binary");
    assert!(
        crate::archive::build_archive(
            &root,
            &root.join("binary.zip"),
            "harness-ultragoal",
            "candidate_review_anchor"
        )
        .expect("binary non-utf8 is allowed when no private path is present")["status"]
            == "pass"
    );

    for (rel, expected) in [
        ("docs/missing.txt", "manifest path does not exist"),
        ("docs", "archive entry is not a file"),
        (".DS_Store", "junk file is not archiveable"),
        ("docs/cache.pyc", "bytecode/cache file is not archiveable"),
    ] {
        if rel == ".DS_Store" {
            std::fs::write(root.join(rel), "junk").expect("junk file");
        }
        if rel == "docs/cache.pyc" {
            std::fs::write(root.join(rel), "bytecode").expect("pyc file");
        }
        manifest(&root, &[rel]);
        assert!(
            crate::archive::build_archive(
                &root,
                &root.join("bad.zip"),
                "harness-ultragoal",
                "candidate_review_anchor"
            )
            .expect_err("archive hygiene failure")
            .contains(expected),
            "{rel}: {expected}"
        );
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("docs/file.txt"), root.join("docs/link.txt"))
            .expect("symlink");
        manifest(&root, &["docs/link.txt"]);
        assert!(
            crate::archive::build_archive(
                &root,
                &root.join("link.zip"),
                "harness-ultragoal",
                "candidate_review_anchor"
            )
            .expect_err("symlink rejected")
            .contains("package path uses symlink")
        );

        std::fs::write(root.join("docs/hard-source.txt"), "hard").expect("hard source");
        std::fs::hard_link(
            root.join("docs/hard-source.txt"),
            root.join("docs/hard-link.txt"),
        )
        .expect("hard link");
        manifest(&root, &["docs/hard-link.txt"]);
        assert!(
            crate::archive::build_archive(
                &root,
                &root.join("hard-link.zip"),
                "harness-ultragoal",
                "candidate_review_anchor"
            )
            .expect_err("hard links rejected at read time")
            .contains("archive read failed")
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup archive edges");
}

#[test]
fn materiality_scope_variants_reject_overclaim_and_missing_evidence() {
    assert_eq!(
        crate::review::materiality::fixture_failures(
            &crate::self_tests::boundaries::workspace_fixtures::temp_root(
                "no-materiality-fixtures"
            )
        ),
        vec!["review_materiality_fixture_dir_missing".to_string()]
    );
    let invalid = crate::review::materiality::value_failures(&json!({"decision":"OTHER"}));
    assert!(invalid.contains(&"materiality_decision_invalid".to_string()));

    let mut full = gate("FULL_SCOPE_MATERIAL_REVIEW_REQUIRED");
    full["reviewers_required"] = json!(["claim-falsifier"]);
    let full_errors = crate::review::materiality::value_failures(&full);
    for expected in [
        "materiality_full_scope_reviewer_set_missing",
        "materiality_reviewer_cannot_authorize_signoff",
        "materiality_anchor_evidence_missing",
        "materiality_reviewer_registry_evidence_missing",
        "materiality_trigger_missing",
    ] {
        assert!(
            full_errors.contains(&expected.to_string()),
            "{expected}: {full_errors:?}"
        );
    }

    let mut delta = gate("DELTA_REVIEW_ALLOWED");
    delta["output_may_be_used_for_material_signoff"] = json!(true);
    delta["reviewers_required"] = json!(["a", "b"]);
    let delta_errors = crate::review::materiality::value_failures(&delta);
    assert!(delta_errors.contains(&"materiality_delta_used_for_signoff".to_string()));
    assert!(delta_errors.contains(&"materiality_delta_reviewer_scope_too_broad".to_string()));

    let mut advisory = gate("ADVISORY_REVIEW_ALLOWED");
    advisory["output_may_be_used_for_material_signoff"] = json!(true);
    advisory["claim_ceiling"]["supported"] = json!(["claim"]);
    let advisory_errors = crate::review::materiality::value_failures(&advisory);
    assert!(advisory_errors.contains(&"materiality_advisory_used_for_signoff".to_string()));
    assert!(advisory_errors.contains(&"materiality_advisory_overclaims_support".to_string()));

    let mut blocked = gate("BLOCKED_BEFORE_REVIEW");
    blocked["reviewers_required"] = json!(["claim-falsifier"]);
    let blocked_errors = crate::review::materiality::value_failures(&blocked);
    assert!(blocked_errors.contains(&"materiality_blocked_launches_reviewers".to_string()));
    assert!(blocked_errors.contains(&"materiality_blocked_without_repair".to_string()));
}
