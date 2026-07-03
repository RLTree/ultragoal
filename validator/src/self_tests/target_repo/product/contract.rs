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

fn copied_product(label: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::support::repo_root();
    let target = crate::self_tests::boundaries::support::temp_root(label);
    copy_dir(
        &root.join("fixtures/target-repo/valid-product-cohesion"),
        &target,
    );
    target
}

fn audit(root: &Path, required: bool) -> Value {
    let (receipt, _) = crate::target_repo::audit_target_repo(
        root,
        "fresh-init",
        "target product cohesion test",
        &[],
        false,
        required,
        None,
    );
    receipt["checks"]["product-cohesion"].clone()
}

fn detail(row: &Value) -> &str {
    row["detail"].as_str().expect("detail")
}

fn journey_path(root: &Path) -> PathBuf {
    root.join("validation_artifacts/product-cohesion/journey-receipt.json")
}

fn read_journey(root: &Path) -> Value {
    crate::json_boundary::read_json(&journey_path(root)).expect("journey")
}

fn write_journey(root: &Path, value: &Value) {
    std::fs::write(
        journey_path(root),
        serde_json::to_vec_pretty(value).expect("journey bytes"),
    )
    .expect("write journey");
}

fn remove_marker(root: &Path) {
    let script = root.join("scripts/check");
    let text = std::fs::read_to_string(&script).expect("script");
    std::fs::write(
        &script,
        text.replace("echo \"harness-check:product-cohesion pass\"\n", ""),
    )
    .expect("remove marker");
}

#[test]
fn target_product_cohesion_reports_marker_signoff_and_unreadable_receipt_edges() {
    let no_marker = copied_product("target-product-no-marker");
    remove_marker(&no_marker);
    let marker_row = audit(&no_marker, false);
    assert_eq!(marker_row["status"], "fail");
    assert!(detail(&marker_row).contains("product cohesion marker missing"));
    std::fs::remove_dir_all(no_marker).expect("cleanup no marker");

    let signoff = copied_product("target-product-signoff");
    let mut journey = read_journey(&signoff);
    journey["review"]["signoff_status"] = json!("blocked");
    write_journey(&signoff, &journey);
    let signoff_row = audit(&signoff, true);
    assert!(detail(&signoff_row).contains("review signoff not pass"));
    std::fs::remove_dir_all(signoff).expect("cleanup signoff");

    let unreadable = copied_product("target-product-hardlink-receipt");
    let link_source = unreadable.join("validation_artifacts/product-cohesion/hardlink-source.json");
    std::fs::copy(journey_path(&unreadable), &link_source).expect("hardlink source");
    std::fs::remove_file(journey_path(&unreadable)).expect("remove journey");
    std::fs::hard_link(&link_source, journey_path(&unreadable)).expect("hardlink journey");
    let unreadable_row = audit(&unreadable, true);
    assert!(detail(&unreadable_row).contains("journey receipt unreadable"));
    std::fs::remove_dir_all(unreadable).expect("cleanup unreadable");
}

#[test]
fn target_product_cohesion_exception_artifacts_fail_and_pass_on_same_surface() {
    let missing = copied_product("target-product-exception-missing");
    let mut journey = read_journey(&missing);
    journey["human_attention_policy"]["expected_interruption_rate"] = json!("frequent");
    journey["human_attention_policy"]["human_review_queue_exception"] = json!({
        "evidence":{"path":"validation_artifacts/product-cohesion/missing-exception.json","digest":crate::self_tests::boundaries::support::sha('a')}
    });
    write_journey(&missing, &journey);
    let missing_row = audit(&missing, true);
    assert!(detail(&missing_row).contains("artifact missing"));
    std::fs::remove_dir_all(missing).expect("cleanup missing exception");

    let malformed = copied_product("target-product-exception-malformed");
    let mut journey = read_journey(&malformed);
    let exception_path =
        malformed.join("validation_artifacts/product-cohesion/human-review-queue-exception.json");
    std::fs::write(&exception_path, "{").expect("write malformed exception");
    journey["human_attention_policy"]["expected_interruption_rate"] = json!("frequent");
    journey["human_attention_policy"]["human_review_queue_exception"] = json!({
        "evidence":{
            "path":"validation_artifacts/product-cohesion/human-review-queue-exception.json",
            "digest":crate::digest::file(&exception_path).expect("malformed exception digest")
        }
    });
    write_journey(&malformed, &journey);
    let malformed_row = audit(&malformed, true);
    assert!(detail(&malformed_row).contains("artifact json malformed"));
    std::fs::remove_dir_all(malformed).expect("cleanup malformed exception");

    let failed = copied_product("target-product-exception-failed");
    let mut journey = read_journey(&failed);
    let exception_path =
        failed.join("validation_artifacts/product-cohesion/human-review-queue-exception.json");
    let payload = json!({
        "schema":"harness-ultragoal.human-review-queue-exception.v1",
        "status":"fail",
        "product_surface_id":journey["product_surface_id"].clone(),
        "intrinsic_human_decision":"human judgment required",
        "scope":"specific product exception scope",
        "owner":"product fitness reviewer",
        "why_automation_is_inappropriate":"requires current human judgment",
        "reviewer_authority":"typed product review authority",
        "review_policy":"review must be same-surface",
        "expires_at":"2026-06-30T00:00:00Z"
    });
    std::fs::write(
        &exception_path,
        serde_json::to_vec_pretty(&payload).expect("failed exception bytes"),
    )
    .expect("write failed exception");
    journey["human_attention_policy"]["expected_interruption_rate"] = json!("frequent");
    journey["human_attention_policy"]["human_review_queue_exception"] = json!({
        "evidence":{
            "path":"validation_artifacts/product-cohesion/human-review-queue-exception.json",
            "digest":crate::digest::file(&exception_path).expect("failed exception digest")
        }
    });
    write_journey(&failed, &journey);
    let failed_row = audit(&failed, true);
    assert!(detail(&failed_row).contains("authority receipt status not pass"));
    std::fs::remove_dir_all(failed).expect("cleanup failed exception");

    let valid = copied_product("target-product-exception-valid");
    let mut journey = read_journey(&valid);
    let surface_id = journey["product_surface_id"].clone();
    let exception_path =
        valid.join("validation_artifacts/product-cohesion/human-review-queue-exception.json");
    let payload = json!({
        "schema":"harness-ultragoal.human-review-queue-exception.v1",
        "status":"pass",
        "product_surface_id":surface_id,
        "intrinsic_human_decision":"human judgment required",
        "scope":"specific product exception scope",
        "owner":"product fitness reviewer",
        "why_automation_is_inappropriate":"requires current human judgment",
        "reviewer_authority":"typed product review authority",
        "review_policy":"review must be same-surface",
        "expires_at":"2026-06-30T00:00:00Z"
    });
    std::fs::write(
        &exception_path,
        serde_json::to_vec_pretty(&payload).expect("exception bytes"),
    )
    .expect("write exception");
    journey["human_attention_policy"]["expected_interruption_rate"] = json!("frequent");
    journey["human_attention_policy"]["human_review_queue_exception"] = json!({
        "evidence":{
            "path":"validation_artifacts/product-cohesion/human-review-queue-exception.json",
            "digest":crate::digest::file(&exception_path).expect("exception digest")
        }
    });
    write_journey(&valid, &journey);
    let valid_row = audit(&valid, true);
    assert_eq!(valid_row["status"], "pass");
    std::fs::remove_dir_all(valid).expect("cleanup valid exception");
}
