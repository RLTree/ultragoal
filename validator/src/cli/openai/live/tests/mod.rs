use super::*;
use crate::cli::openai::call;
use crate::self_tests::boundaries::support::sha;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

mod errors;
mod success;

#[test]
fn live_execute_fails_closed_before_network_when_input_digest_mismatches() {
    let root = crate::self_tests::openai::prepare_root(
        "openai-live-no-network",
        &[
            ".gitignore",
            "docs/openai-key-policy.json",
            "docs/openai-provider-policy.json",
        ],
    );
    let raw = [
        "openai",
        "call",
        "prove",
        "--mode",
        "openai_live",
        "--input-digest",
        &sha('a'),
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>();
    let command = call::parse(&raw).expect("parse live call");
    let budget = live_budget(&root);

    let observation = execute(&root, &command, &budget).expect("live observation");
    assert!(
        observation
            .failures
            .contains(&"openai_live_prompt_input_digest_mismatch".to_string())
    );
    assert!(
        observation
            .failures
            .contains(&"openai_live_temp_read_failed".to_string())
    );
    assert!(
        observation
            .failures
            .iter()
            .any(|failure| failure.starts_with("openai_live_http_status_not_success:"))
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn live_execute_fails_closed_when_key_policy_is_unavailable() {
    let root = crate::self_tests::openai::prepare_root(
        "openai-live-key-unavailable",
        &["docs/openai-provider-policy.json"],
    );
    std::fs::remove_file(root.join(crate::cli::openai::config::DEFAULT_POLICY))
        .expect("remove key policy");
    let command = live_command();
    let budget = live_budget(&root);
    let observation = execute(&root, &command, &budget).expect("observation");
    assert!(
        observation
            .failures
            .contains(&"openai_key_policy_not_passing".to_string())
    );
    assert!(
        observation
            .failures
            .contains(&"openai_live_temp_read_failed".to_string())
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn live_http_header_usage_latency_and_temp_helpers_are_typed() {
    let body = request_body("gpt-test");
    assert_eq!(body["model"], "gpt-test");
    assert_eq!(
        timeout_seconds(&crate::cli::openai::budget::BudgetSelection {
            policy_digest: sha('b'),
            budget_class: "source_live_low".to_string(),
            selected: json!({"timeout_ms": 4500}),
            failures: Vec::new(),
        }),
        4
    );

    let parsed = json!({
        "id": "resp_123",
        "usage": {
            "input_tokens": 5,
            "output_tokens": 7,
            "total_tokens": 12
        }
    });
    assert!(http_failures(Some("200 0.123"), &parsed).is_empty());
    assert_eq!(
        http_failures(Some("500 0.123"), &json!({})),
        vec![
            "openai_live_http_status_not_success:500".to_string(),
            "openai_live_response_missing_id".to_string()
        ]
    );
    assert_eq!(
        request_id("X-Request-Id: req_header\n", &parsed),
        "req_header"
    );
    assert_eq!(request_id("", &parsed), "resp_123");
    assert_eq!(usage(&parsed, "total_tokens"), 12);
    assert_eq!(usage(&json!({}), "total_tokens"), 0);
    assert_eq!(latency_ms(Some("200 0.123")), 123);
    assert_eq!(latency_ms(Some("bad")), 0);

    let paths = TempPaths::new();
    let mut failures = Vec::new();
    write_temp(&paths.response, br#"{"id":"resp_temp"}"#, &mut failures);
    assert_eq!(
        read_temp(&paths.response, &mut failures),
        r#"{"id":"resp_temp"}"#
    );
    let _ = std::fs::remove_file(&paths.response);
    assert_eq!(read_temp(&paths.response, &mut failures), "");
    assert!(failures.contains(&"openai_live_temp_read_failed".to_string()));
    let dir_path = std::env::temp_dir();
    write_temp(&dir_path, b"cannot write to directory", &mut failures);
    assert!(failures.contains(&"openai_live_temp_write_failed".to_string()));
    paths.cleanup();
}

#[test]
fn live_curl_runner_records_spawn_success_and_exit_failure_without_network() {
    let paths = TempPaths::new();
    let echo_script = temp_script("openai-live-echo", "cat\n");
    let mut success_failures = Vec::new();
    let output = run_curl_with_program(
        echo_script.to_str().expect("script path"),
        "test-key",
        &paths,
        3,
        &mut success_failures,
    )
    .expect("script echoes config");
    assert!(output.contains("https://api.openai.com/v1/responses"));
    assert!(success_failures.is_empty(), "{success_failures:?}");

    let fail_script = temp_script("openai-live-fail", "exit 7\n");
    let mut exit_failures = Vec::new();
    let _ = run_curl_with_program(
        fail_script.to_str().expect("fail script path"),
        "test-key",
        &paths,
        3,
        &mut exit_failures,
    );
    assert!(exit_failures.contains(&"openai_live_curl_exit_failed".to_string()));

    let mut spawn_failures = Vec::new();
    assert!(
        run_curl_with_program(
            "ultragoal-openai-live-command-that-does-not-exist",
            "test-key",
            &paths,
            3,
            &mut spawn_failures
        )
        .is_none()
    );
    assert!(spawn_failures.contains(&"openai_live_curl_spawn_failed".to_string()));
    paths.cleanup();
    let _ = std::fs::remove_file(echo_script);
    let _ = std::fs::remove_file(fail_script);
}

fn temp_script(label: &str, body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "{label}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::write(&path, format!("#!/bin/sh\n{body}")).expect("script");
    make_executable(&path);
    path
}

fn live_command() -> call::CallCommand {
    call::CallCommand {
        receipt: "validation_artifacts/openai/live.json".into(),
        provider_mode: "openai_live".to_string(),
        model_identity: "gpt-5.4-nano".to_string(),
        endpoint_api_family: "responses".to_string(),
        purpose: "boundary".to_string(),
        schema_id: "schema".to_string(),
        input_digest: crate::digest::ZERO.to_string(),
        output_digest: crate::digest::ZERO.to_string(),
        provider_policy: crate::cli::openai::budget::DEFAULT_POLICY.into(),
        budget_class: "source_live_low".to_string(),
    }
}

fn live_budget(root: &Path) -> crate::cli::openai::budget::BudgetSelection {
    crate::cli::openai::budget::load(
        root,
        Path::new(crate::cli::openai::budget::DEFAULT_POLICY),
        "source_live_low",
        "openai_live",
    )
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(path, permissions).expect("chmod");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {
    panic!("openai live process tests require unix executable permissions");
}
