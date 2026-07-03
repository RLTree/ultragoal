use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("copy target");
    for entry in std::fs::read_dir(from).expect("read fixture dir") {
        let entry = entry.expect("dir entry");
        let ty = entry.file_type().expect("file type");
        let dest = to.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).expect("copy fixture file");
        }
    }
}

fn copied_fixture(label: &str, fixture: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let target = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    copy_dir(&root.join(fixture), &target);
    target
}

fn audit_product(root: &Path, required: bool) -> Value {
    let (receipt, _) = crate::target_repo::audit_target_repo(
        root,
        "fresh-init",
        "target audit test",
        &[],
        false,
        required,
        None,
    );
    receipt["checks"]["product-cohesion"].clone()
}

fn product_detail(row: &Value) -> &str {
    row["detail"].as_str().expect("detail")
}

fn read_journey(root: &Path) -> Value {
    crate::json_boundary::read_json(
        &root.join("validation_artifacts/product-cohesion/journey-receipt.json"),
    )
    .expect("journey json")
}

fn write_journey(root: &Path, value: &Value) {
    std::fs::write(
        root.join("validation_artifacts/product-cohesion/journey-receipt.json"),
        serde_json::to_vec_pretty(value).expect("journey bytes"),
    )
    .expect("write journey");
}

fn write_exception(root: &Path, payload: Value) {
    let path = root.join("validation_artifacts/product-cohesion/human-review-queue-exception.json");
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&payload).expect("payload bytes"),
    )
    .expect("write exception");
    let mut journey = read_journey(root);
    journey["human_attention_policy"]["human_review_queue_exception"]["evidence"]["digest"] =
        json!(crate::digest::file(&path).expect("exception digest"));
    write_journey(root, &journey);
}

fn args(root: PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}

#[test]
fn product_cohesion_audit_reports_missing_marker_and_malformed_receipt() {
    let empty = crate::self_tests::boundaries::workspace_fixtures::temp_root("product-empty");
    std::fs::create_dir_all(&empty).expect("empty target");
    let optional = audit_product(&empty, false);
    assert_eq!(optional["status"], "not_applicable");
    assert!(product_detail(&optional).contains("not requested"));
    let required = audit_product(&empty, true);
    assert_eq!(required["status"], "blocked");
    assert!(product_detail(&required).contains("docs/product-cohesion.md"));
    std::fs::remove_dir_all(empty).expect("cleanup empty");

    let no_marker = copied_fixture(
        "product-no-marker",
        "fixtures/target-repo/valid-product-cohesion",
    );
    let script = no_marker.join("scripts/check");
    let text = std::fs::read_to_string(&script).expect("script");
    std::fs::write(
        &script,
        text.replace("echo \"harness-check:product-cohesion pass\"\n", ""),
    )
    .expect("remove marker");
    let marker_row = audit_product(&no_marker, true);
    assert_eq!(marker_row["status"], "blocked");
    assert!(product_detail(&marker_row).contains("gate marker"));
    std::fs::remove_dir_all(no_marker).expect("cleanup marker");

    let malformed = copied_fixture(
        "product-malformed",
        "fixtures/target-repo/valid-product-cohesion",
    );
    std::fs::write(
        malformed.join("validation_artifacts/product-cohesion/journey-receipt.json"),
        "{",
    )
    .expect("malformed journey");
    let malformed_row = audit_product(&malformed, true);
    assert_eq!(malformed_row["status"], "blocked");
    assert!(product_detail(&malformed_row).contains("journey receipt malformed"));
    std::fs::remove_dir_all(malformed).expect("cleanup malformed");
}

#[test]
fn product_cohesion_human_attention_exception_branches_are_specific() {
    let no_exception = copied_fixture(
        "product-no-exception",
        "fixtures/target-repo/valid-product-cohesion",
    );
    let mut journey = read_journey(&no_exception);
    journey["human_attention_policy"]["expected_interruption_rate"] = json!("frequent");
    journey["human_attention_policy"]["human_review_queue_exception"] = Value::Null;
    write_journey(&no_exception, &journey);
    let row = audit_product(&no_exception, true);
    assert!(product_detail(&row).contains("requires human review queue exception"));
    std::fs::remove_dir_all(no_exception).expect("cleanup no exception");

    let exception = copied_fixture(
        "product-exception-branches",
        "fixtures/target-repo/red/exception-product-cohesion",
    );
    let schema_row = audit_product(&exception, true);
    assert!(product_detail(&schema_row).contains("evidence schema mismatch"));

    let surface_id = read_journey(&exception)["product_surface_id"].clone();
    write_exception(
        &exception,
        json!({
            "schema": "harness-ultragoal.human-review-queue-exception.v1",
            "status": "fail",
            "product_surface_id": surface_id
        }),
    );
    let status_row = audit_product(&exception, true);
    assert!(product_detail(&status_row).contains("authority receipt status not pass"));

    write_exception(
        &exception,
        json!({
            "schema": "harness-ultragoal.human-review-queue-exception.v1",
            "status": "pass",
            "product_surface_id": "other-surface"
        }),
    );
    let mismatch_row = audit_product(&exception, true);
    assert!(product_detail(&mismatch_row).contains("product surface mismatch"));

    write_exception(
        &exception,
        json!({
            "schema": "harness-ultragoal.human-review-queue-exception.v1",
            "status": "pass",
            "product_surface_id": surface_id,
            "intrinsic_human_decision": "short"
        }),
    );
    let field_row = audit_product(&exception, true);
    assert!(product_detail(&field_row).contains("missing substantive"));
    std::fs::remove_dir_all(exception).expect("cleanup exception");
}

#[test]
fn command_dispatch_returns_typed_exit_codes_for_fail_closed_paths() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let control_root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("control-dispatch");
    std::fs::create_dir_all(&control_root).expect("control dir");
    std::fs::write(
        control_root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"version":"0.0.0-test","resources":[]}))
            .expect("control manifest"),
    )
    .expect("write control manifest");
    let control_receipt =
        control_root.join("validation_artifacts/cli/update-goal-eligibility.json");
    let control_code = crate::command_run::run_with_exit_code(args(
        control_root.clone(),
        &[
            "update-goal",
            "eligibility",
            "--receipt",
            control_receipt.to_str().expect("control path"),
        ],
    ))
    .expect("control command");
    assert_eq!(control_code, 1);
    assert_eq!(
        crate::json_boundary::read_json(&control_receipt).expect("control receipt")["status"],
        "fail"
    );

    let control_dir =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("performance-dispatch");
    std::fs::create_dir_all(&control_dir).expect("performance dir");
    let performance_receipt = control_dir.join("performance.json");
    let performance_code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "performance",
            "prove",
            "--receipt",
            performance_receipt.to_str().expect("performance path"),
        ],
    ))
    .expect("performance command");
    assert_eq!(performance_code, 0);
    assert_eq!(
        crate::json_boundary::read_json(&performance_receipt).expect("performance receipt")["status"],
        "pass"
    );

    let review_code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "review-round",
            "verify",
            "--receipt",
            "fixtures/review-round/valid/review-round-receipt.json",
            "--validator-receipt",
            "fixtures/review-round/anchors/validator-receipt.json",
            "--review-target-receipt",
            "fixtures/review-round/anchors/review-target-receipt.json",
            "--archive-receipt",
            "fixtures/review-round/anchors/archive-receipt.json",
        ],
    ))
    .expect("review command");
    assert_eq!(review_code, 1);
    std::fs::remove_dir_all(control_root).expect("cleanup control dispatch root");
    std::fs::remove_dir_all(control_dir).expect("cleanup command receipts");
}
