use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn args(root: PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}

fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos()
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("path under root")
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn command_run_accepts_current_review_round_anchors() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let out_dir = root
        .join("validation_artifacts/review")
        .join(format!("review-round-command-{}", stamp()));
    std::fs::create_dir_all(&out_dir).expect("review round output dir");
    let package_digest = crate::package::inventory::package_digest(&root).expect("package digest");
    let validator_path = out_dir.join("validator-receipt.json");
    write_json(
        &validator_path,
        &json!({
            "schema":"harness-ultragoal.validator-receipt.v1",
            "status":"pass",
            "run_id":"ultragoal-audit-command-test",
            "target_revision":{"kind":"package_digest","value":package_digest}
        }),
    );
    let validator_digest = crate::digest::file(&validator_path).expect("validator digest");
    let review_target_path = out_dir.join("review-target-receipt.json");
    let review_target_digest = crate::self_tests::boundaries::workspace_fixtures::sha('b');
    write_json(
        &review_target_path,
        &json!({
            "status":"pass",
            "package_digest":package_digest,
            "validator_receipt":{"digest":validator_digest},
            "review_target_digest":review_target_digest
        }),
    );
    let archive_path = out_dir.join("archive-receipt.json");
    let archive_digest = crate::self_tests::boundaries::workspace_fixtures::sha('c');
    write_json(
        &archive_path,
        &json!({
            "status":"pass",
            "archive":{"digest":archive_digest},
            "source":{"package_digest":package_digest}
        }),
    );

    let rel_validator = rel(&root, &validator_path);
    let rel_review_target = rel(&root, &review_target_path);
    let rel_archive = rel(&root, &archive_path);
    let review_target_file_digest =
        crate::digest::file(&review_target_path).expect("target file digest");
    let archive_file_digest = crate::digest::file(&archive_path).expect("archive file digest");
    let mut receipt = crate::json_boundary::read_json(
        &root.join("fixtures/review-round/valid/review-round-receipt.json"),
    )
    .expect("fixture receipt");
    receipt["validator_receipt"] = json!({
        "path": rel_validator,
        "digest": validator_digest,
        "run_id": "ultragoal-audit-command-test",
        "package_digest": package_digest
    });
    receipt["review_target"] = json!({"path": rel_review_target, "digest": review_target_digest});
    receipt["archive"] = json!({"path": rel_archive, "digest": archive_digest});
    receipt["materiality_gate"]["anchors_checked"] = json!([
        {"path": rel_validator, "digest": validator_digest},
        {"path": rel_review_target, "digest": review_target_file_digest},
        {"path": rel_archive, "digest": archive_file_digest}
    ]);
    for row in receipt["reviewers"].as_array_mut().expect("reviewer rows") {
        refresh_current_artifact_refs(&root, row, "proof_anchors_checked");
        refresh_current_artifact_refs(&root, row, "evidence_artifacts_checked");
        row["validator_receipt_digest"] = json!(validator_digest);
        row["review_target_digest"] = json!(review_target_digest);
        row["archive_digest"] = json!(archive_digest);
        replace_array_refs(
            row,
            "proof_anchors_checked",
            &rel_validator,
            &validator_digest,
        );
        replace_array_refs(
            row,
            "proof_anchors_checked",
            &rel_review_target,
            &review_target_digest,
        );
        replace_array_refs(row, "proof_anchors_checked", &rel_archive, &archive_digest);
        replace_array_refs(
            row,
            "evidence_artifacts_checked",
            &rel_validator,
            &validator_digest,
        );
        replace_array_refs(
            row,
            "evidence_artifacts_checked",
            &rel_review_target,
            &review_target_file_digest,
        );
        replace_array_refs(
            row,
            "evidence_artifacts_checked",
            &rel_archive,
            &archive_file_digest,
        );
        replace_repair_evidence(row, &rel_validator, &validator_digest);
        crate::self_tests::command::round_receipts::sync_persona_refs(&root, row);
        crate::self_tests::command::round_receipts::bind_product_fitness(&root, row);
    }
    let receipt_path = out_dir.join("review-round-receipt.json");
    write_json(&receipt_path, &receipt);
    let observability_receipt = rel(&root, &out_dir.join("review-round-observability.json"));

    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "review-round",
            "verify",
            "--receipt",
            receipt_path.to_str().expect("receipt"),
            "--validator-receipt",
            validator_path.to_str().expect("validator"),
            "--review-target-receipt",
            review_target_path.to_str().expect("target"),
            "--archive-receipt",
            archive_path.to_str().expect("archive"),
            "--observability-receipt",
            &observability_receipt,
        ],
    ))
    .expect("review round command");
    assert_eq!(code, 0);
    let observability =
        crate::json_boundary::read_json(&root.join(&observability_receipt)).expect("observability");
    assert_eq!(observability["status"], "pass");
    assert_eq!(observability["operation"], "review-round.verify");
    assert_eq!(
        observability["check_id"],
        "review-round-verify-observability-binding"
    );
    assert_eq!(
        observability["claim_id"],
        "review_round_source_local_observability"
    );
    std::fs::remove_dir_all(out_dir).expect("cleanup review round command");
}

fn replace_array_refs(row: &mut Value, key: &str, path: &str, digest: &str) {
    for anchor in row[key].as_array_mut().expect("artifact refs") {
        if same_anchor_file(anchor, path) {
            anchor["path"] = json!(path);
            anchor["digest"] = json!(digest);
        }
    }
}

fn refresh_current_artifact_refs(root: &Path, row: &mut Value, key: &str) {
    for anchor in row[key].as_array_mut().expect("artifact refs") {
        let Some(path) = anchor.get("path").and_then(Value::as_str) else {
            continue;
        };
        let artifact_path = root.join(path);
        if artifact_path.is_file()
            && let Ok(digest) = crate::digest::file(&artifact_path)
        {
            anchor["digest"] = json!(digest);
        }
    }
}

fn replace_repair_evidence(row: &mut Value, path: &str, digest: &str) {
    for repair in row["required_next_repairs"]
        .as_array_mut()
        .expect("repair rows")
    {
        if same_anchor_file(&repair["evidence"], path) {
            repair["evidence"]["path"] = json!(path);
            repair["evidence"]["digest"] = json!(digest);
        }
    }
}

fn same_anchor_file(value: &Value, path: &str) -> bool {
    value
        .get("path")
        .and_then(Value::as_str)
        .is_some_and(|old| old.ends_with(path.rsplit('/').next().expect("anchor file name")))
}

#[test]
fn review_round_artifact_refresh_skips_malformed_refs_and_updates_current_paths() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let mut row = json!({
        "refs":[
            {},
            {"path":"plugin-manifest-draft.json","digest":crate::digest::ZERO}
        ]
    });
    refresh_current_artifact_refs(&root, &mut row, "refs");
    assert!(row["refs"][0].get("digest").is_none());
    assert_ne!(row["refs"][1]["digest"], crate::digest::ZERO);
}
