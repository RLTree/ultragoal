use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn review_round_anchor_reader_rejects_same_basename_alternate_paths() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("review-anchor-alternate");
    std::fs::create_dir_all(root.join("alternate")).expect("alternate");
    for (name, value) in anchor_values() {
        write_json(&root.join("alternate").join(name), &value);
    }
    let err = read_error(
        &root,
        "alternate/validator-receipt.json",
        "alternate/review-target-receipt.json",
        "alternate/archive-receipt.json",
    );
    assert!(
        err.contains("review_round_trusted_anchor_source_unavailable"),
        "{err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup alternate anchors");
}

#[test]
fn review_round_anchor_reader_rejects_package_root_escape() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("review-anchor-escape");
    let err = read_error(
        &root,
        "../validator-receipt.json",
        "validation_artifacts/review/review-target-receipt.json",
        "validation_artifacts/review/candidate-archive-receipt.json",
    );
    assert_eq!(err, "review_round_anchor_path_invalid");
}

#[cfg(unix)]
#[test]
fn review_round_anchor_reader_rejects_ancestor_symlink() {
    use std::os::unix::fs::symlink;

    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("review-anchor-link");
    let outside =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("review-anchor-outside");
    std::fs::create_dir_all(root.join("validation_artifacts")).expect("artifact parent");
    write_json(
        &outside.join("validator-receipt.json"),
        &anchor_values()[0].1,
    );
    for (name, value) in live_review_anchor_values() {
        write_json(&root.join("validation_artifacts/review").join(name), &value);
    }
    symlink(&outside, root.join("validation_artifacts/ultragoal-audit")).expect("ancestor symlink");
    let err = read_error(
        &root,
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "validation_artifacts/review/review-target-receipt.json",
        "validation_artifacts/review/candidate-archive-receipt.json",
    );
    assert!(err.contains("review_round_anchor_path_invalid"), "{err}");
    std::fs::remove_dir_all(root).expect("cleanup link root");
    std::fs::remove_dir_all(outside).expect("cleanup link outside");
}

#[test]
fn live_anchor_semantics_reject_empty_run_and_placeholder_identities() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("review-anchor-live");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let package = crate::package::inventory::package_digest(&root).expect("empty package digest");
    let validator = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    write_json(
        &validator,
        &json!({"schema":"harness-ultragoal.validator-receipt.v1","status":"pass","run_id":"","target_revision":{"kind":"package_digest","value":package}}),
    );
    let validator_digest = crate::digest::file(&validator).expect("validator digest");
    write_json(
        &root.join("validation_artifacts/review/review-target-receipt.json"),
        &json!({"schema":"harness-ultragoal.review-target-receipt.v1","status":"pass","package_digest":package,"validator_receipt":{"path":"validation_artifacts/ultragoal-audit/validator-receipt.json","digest":validator_digest},"review_target_digest":crate::self_tests::boundaries::workspace_fixtures::sha('b')}),
    );
    write_json(
        &root.join("validation_artifacts/review/candidate-archive-receipt.json"),
        &json!({"schema":"harness-ultragoal.distribution-archive-receipt.v1","status":"pass","archive_purpose":"candidate_review_anchor","source":{"package_digest":package},"archive":{"digest":crate::self_tests::boundaries::workspace_fixtures::sha('c')}}),
    );
    let values = crate::review::round::anchor::values::AnchorValues::read(
        Some(&root),
        &validator,
        &root.join("validation_artifacts/review/review-target-receipt.json"),
        &root.join("validation_artifacts/review/candidate-archive-receipt.json"),
    )
    .expect("canonical live paths are readable but semantically blocked");
    assert!(
        values
            .source_errors
            .iter()
            .any(|error| error == "review_round_validator_run_invalid"),
        "{:?}",
        values.source_errors
    );
    assert!(
        values
            .source_errors
            .iter()
            .any(|error| error == "review_round_anchor_placeholder_digest"),
        "{:?}",
        values.source_errors
    );
    write_json(
        &root.join("validation_artifacts/review/review-target-receipt.json"),
        &json!({"schema":"harness-ultragoal.review-target-receipt.v1","status":"pass","package_digest":crate::self_tests::boundaries::workspace_fixtures::sha('9'),"validator_receipt":{"path":"validation_artifacts/ultragoal-audit/validator-receipt.json","digest":validator_digest},"review_target_digest":crate::self_tests::boundaries::workspace_fixtures::sha('b')}),
    );
    let mismatched = crate::review::round::anchor::values::AnchorValues::read(
        Some(&root),
        &validator,
        &root.join("validation_artifacts/review/review-target-receipt.json"),
        &root.join("validation_artifacts/review/candidate-archive-receipt.json"),
    )
    .expect("canonical live paths remain readable for semantic rejection");
    assert!(
        mismatched
            .source_errors
            .iter()
            .any(|error| error == "review_round_anchor_candidate_mismatch"),
        "{:?}",
        mismatched.source_errors
    );
    std::fs::remove_dir_all(root).expect("cleanup live anchors");
}

#[test]
fn anchor_reader_rejects_special_file_with_stable_non_echoing_error() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("review-anchor-special");
    let validator = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    std::fs::create_dir_all(&validator).expect("directory special file");
    for (name, value) in live_review_anchor_values() {
        write_json(&root.join("validation_artifacts/review").join(name), &value);
    }
    let err = read_error(
        &root,
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "validation_artifacts/review/review-target-receipt.json",
        "validation_artifacts/review/candidate-archive-receipt.json",
    );
    assert_eq!(err, "review_round_anchor_not_regular");
    assert!(!err.contains("validator-receipt"));
    std::fs::remove_dir_all(root).expect("cleanup special anchor");
}

#[cfg(unix)]
#[test]
fn anchor_reader_rejects_leaf_symlink_and_hardlink_without_echo() {
    use std::os::unix::fs::symlink;

    for attack in ["leaf-symlink", "hardlink"] {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(attack);
        let validator = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
        let attacker = root.join("attacker-controlled-secret.json");
        write_json(&attacker, &anchor_values()[0].1);
        std::fs::create_dir_all(validator.parent().expect("validator parent"))
            .expect("validator parent");
        match attack {
            "leaf-symlink" => symlink(&attacker, &validator).expect("leaf symlink"),
            "hardlink" => std::fs::hard_link(&attacker, &validator).expect("hardlink"),
            _ => unreachable!(),
        }
        for (name, value) in live_review_anchor_values() {
            write_json(&root.join("validation_artifacts/review").join(name), &value);
        }
        let err = read_error(
            &root,
            "validation_artifacts/ultragoal-audit/validator-receipt.json",
            "validation_artifacts/review/review-target-receipt.json",
            "validation_artifacts/review/candidate-archive-receipt.json",
        );
        let expected = match attack {
            "leaf-symlink" => "review_round_anchor_path_invalid",
            "hardlink" => "review_round_anchor_not_regular",
            _ => unreachable!(),
        };
        assert_eq!(err, expected, "{attack}: {err}");
        assert!(
            !err.contains("attacker-controlled-secret"),
            "{attack}: {err}"
        );
        std::fs::remove_dir_all(root).expect("cleanup linked anchor");
    }
}

fn read_error(root: &Path, validator: &str, review: &str, archive: &str) -> String {
    match crate::review::round::anchor::values::AnchorValues::read(
        Some(root),
        &root.join(validator),
        &root.join(review),
        &root.join(archive),
    ) {
        Ok(_) => panic!("untrusted anchor source was accepted"),
        Err(error) => error,
    }
}

fn anchor_values() -> Vec<(&'static str, Value)> {
    let sha = crate::self_tests::boundaries::workspace_fixtures::sha;
    vec![
        (
            "validator-receipt.json",
            json!({"schema":"harness-ultragoal.validator-receipt.v1","status":"pass","run_id":"run","target_revision":{"kind":"package_digest","value":sha('1')}}),
        ),
        (
            "review-target-receipt.json",
            json!({"schema":"harness-ultragoal.review-target-receipt.v1","status":"pass","review_target_digest":sha('2')}),
        ),
        (
            "archive-receipt.json",
            json!({"schema":"harness-ultragoal.distribution-archive-receipt.v1","status":"pass","archive":{"digest":sha('3')}}),
        ),
    ]
}

fn live_review_anchor_values() -> Vec<(&'static str, Value)> {
    let values = anchor_values();
    vec![
        ("review-target-receipt.json", values[1].1.clone()),
        ("candidate-archive-receipt.json", values[2].1.clone()),
    ]
}
