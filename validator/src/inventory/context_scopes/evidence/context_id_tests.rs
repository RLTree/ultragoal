use super::valid_record;
use serde_json::{Value, json};

const FILE_NAME: &str = "LEASE-N02-TEST-CONTEXT-001.json";

fn worker_result(candidate_identity: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "worker": "test-worker",
        "lease_id": "LEASE-N02-TEST-CONTEXT-001",
        "context_id": "sha256:test-context",
        "candidate_identity": candidate_identity,
        "base_state": {},
        "final_state": {},
        "touched_paths": [],
        "touched_semantics": [],
        "generated_outputs": [],
        "fixtures": [],
        "effects": [],
        "requirements": [],
        "dependency_nodes": [],
        "changes": [],
        "commands_and_tests": [],
        "artifacts": [],
        "findings": [],
        "unresolved_dependencies": [],
        "requested_root_changes": [],
        "limitations": [],
        "no_claim_statement": "This worker does not claim readiness, release, or completion."
    }))
    .unwrap()
}

#[test]
fn nested_private_strings_fail_worker_record_validation() {
    for candidate_identity in [
        json!({"repository_root": "/Users/operator/private-project"}),
        json!({"nested": [{"command": "cargo test --target-dir /private/operator/build"}]}),
        json!({"nested": {"review_session_id": "019fa4db-318f-7631-947b-a9747ea9ef9a"}}),
        json!({"nested": [{"task_id": "private-task-123"}]}),
        json!({"nested": {"authorization": "Bearer SECRET_VALUE_CANARY"}}),
        json!({"nested": {"api_key": "opaque-sensitive-canary"}}),
        json!({"nested": {"github_token": "opaque-sensitive-canary"}}),
        json!({"nested": {"client_secret": "opaque-sensitive-canary"}}),
        json!({"nested": {"signing_private_key": "opaque-sensitive-canary"}}),
        json!({"nested": {"api_key": {"value": "opaque-sensitive-canary"}}}),
        json!({"nested": {"api_key": 123456789}}),
        json!({"nested": {"task_id": 12345}}),
        json!({"nested": {"authorization": false}}),
        json!({"nested": {"client_secret": null}}),
        json!({"nested": {"documentation_url": "https://operator:private@example.com/reference"}}),
        json!({"nested": {"sk-SECRET_KEY_CANARY": true}}),
    ] {
        assert!(!valid_record(&worker_result(candidate_identity), FILE_NAME));
    }
}

#[test]
fn public_network_and_json_pointer_strings_remain_valid() {
    let bytes = worker_result(json!({
        "documentation_url": "https://docs.example.com/users/operator-guide",
        "service_domain": "api.example.com",
        "public_json_pointer": "/status/current"
    }));
    assert!(valid_record(&bytes, FILE_NAME));
}
