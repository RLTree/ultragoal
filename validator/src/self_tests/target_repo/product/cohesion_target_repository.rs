use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const JOURNEY_RECEIPT: &str = "validation_artifacts/product-cohesion/journey-receipt.json";
const HUMAN_REVIEW_EXCEPTION: &str =
    "validation_artifacts/product-cohesion/human-review-queue-exception.json";

fn copy_product_cohesion_repository(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("copy target");
    for entry in std::fs::read_dir(from).expect("read fixture dir") {
        let entry = entry.expect("dir entry");
        let ty = entry.file_type().expect("file type");
        let dest = to.join(entry.file_name());
        if ty.is_dir() {
            copy_product_cohesion_repository(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).expect("copy fixture file");
        }
    }
}

pub(super) fn product_cohesion_repository_from_fixture(label: &str, fixture: &str) -> PathBuf {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    let target = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    copy_product_cohesion_repository(&root.join(fixture), &target);
    target
}

pub(super) fn audit_product_cohesion(root: &Path, required: bool) -> Value {
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

pub(super) fn product_cohesion_detail(row: &Value) -> &str {
    row["detail"].as_str().expect("detail")
}

pub(super) fn read_product_journey_receipt(root: &Path) -> Value {
    crate::json_boundary::read_json(&root.join(JOURNEY_RECEIPT)).expect("journey json")
}

pub(super) fn write_product_journey_receipt(root: &Path, value: &Value) {
    std::fs::write(
        root.join(JOURNEY_RECEIPT),
        serde_json::to_vec_pretty(value).expect("journey bytes"),
    )
    .expect("write journey");
}

pub(super) fn write_human_review_exception_receipt(root: &Path, payload: Value) {
    let path = root.join(HUMAN_REVIEW_EXCEPTION);
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&payload).expect("payload bytes"),
    )
    .expect("write exception");
    let mut journey = read_product_journey_receipt(root);
    journey["human_attention_policy"]["human_review_queue_exception"]["evidence"]["digest"] =
        json!(crate::digest::file(&path).expect("exception digest"));
    write_product_journey_receipt(root, &journey);
}

pub(super) fn command_args_for_product_cohesion_dispatch(
    root: PathBuf,
    raw: &[&str],
) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}
