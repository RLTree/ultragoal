#[test]
fn live_execute_success_path_is_typed_without_network() {
    let root = crate::self_tests::openai::prepare_root(
        "openai-live-fake-success",
        &[
            ".gitignore",
            "docs/openai-key-policy.json",
            "docs/openai-provider-policy.json",
        ],
    );
    let command = super::live_command();
    let budget = super::live_budget(&root);
    let script = super::temp_script(
        "openai-live-success",
        "cfg=$(cat)\nresponse=$(printf '%s\\n' \"$cfg\" | awk -F'\\\"' '/output =/ {print $2}')\nheaders=$(printf '%s\\n' \"$cfg\" | awk -F'\\\"' '/dump-header =/ {print $2}')\nprintf '{\"id\":\"resp_fake\",\"usage\":{\"input_tokens\":3,\"output_tokens\":4,\"total_tokens\":7}}' > \"$response\"\nprintf 'X-Request-Id: req_fake\\nx-ratelimit-limit-requests: 99\\n' > \"$headers\"\nprintf '200 0.010'\n",
    );
    let observation =
        super::super::execute_with_program(&root, &command, &budget, script.to_str().unwrap())
            .expect("live");
    assert!(
        observation.failures.is_empty(),
        "{:?}",
        observation.failures
    );
    assert_eq!(observation.request_id, "req_fake");
    assert_eq!(observation.total_tokens, 7);
    assert_eq!(observation.latency_ms, 10);
    assert!(observation.rate_limit_observed);
    let _ = std::fs::remove_file(script);
    std::fs::remove_dir_all(root).expect("cleanup");
}
