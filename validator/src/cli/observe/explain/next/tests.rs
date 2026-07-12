use super::plan;
use crate::cli::observe::types::{ObserveCommand, ObserveOperation};
use serde_json::json;

fn command() -> ObserveCommand {
    ObserveCommand {
        operation: ObserveOperation::ExplainNext,
        receipt: None,
        query: None,
        run_id: None,
        correlation_id: None,
        claim_id: None,
        check_id: None,
        law_id: None,
        target_command: None,
        target_family: None,
        row_limit: 100,
        byte_limit: 4096,
        timeout_ms: 1000,
    }
}

#[test]
fn next_plan_is_an_explicit_unavailable_blocker_with_no_supported_claims() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("next-unavailable");
    std::fs::create_dir_all(&root).expect("root");
    let state = json!({"candidate_digest":"sha256:current"});
    let value = plan(&root, &command(), &state).expect("bounded unavailable plan");

    assert_eq!(value["status"], "fail");
    assert_eq!(value["target_row"], "HCT-OBSERVE");
    assert_eq!(
        value["why_failed"],
        "HCT-OBSERVE successor catalog unavailable/not adopted"
    );
    assert_eq!(value["supported_claims"], json!([]));
    assert_eq!(value["required_query_commands"], json!([]));
    assert!(!root.join("validation_artifacts").exists());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn next_plan_is_identical_across_static_inventory_mutations_and_writes_nothing() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("next-static-bait");
    let directory = root.join("docs/generated/observability");
    std::fs::create_dir_all(&directory).expect("directory");
    let path = directory.join("command-inventory.json");
    let state = json!({"candidate_digest":"sha256:current"});

    std::fs::write(&path, "SECRET_CANARY").expect("bait");
    let first = plan(&root, &command(), &state).expect("first plan");
    std::fs::write(&path, [0xff, 0xfe]).expect("invalid bytes");
    let second = plan(&root, &command(), &state).expect("second plan");
    std::fs::remove_file(&path).expect("remove bait");
    let missing = plan(&root, &command(), &state).expect("missing plan");

    assert_eq!(first, second);
    assert_eq!(second, missing);
    assert!(!first.to_string().contains("SECRET_CANARY"));
    assert!(!root.join("validation_artifacts").exists());
    std::fs::remove_dir_all(root).expect("cleanup");
}
