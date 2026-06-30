#[test]
fn policy_parser_handles_ignored_quoted_missing_and_malformed_env() {
    assert_eq!(
        super::parse_api_key("ignored\nOPENAI_API_KEY='test-key-redacted'\n").as_deref(),
        Some("test-key-redacted")
    );
    assert_eq!(
        super::parse_api_key("export OPENAI_API_KEY=\"test-key-redacted\"").as_deref(),
        Some("test-key-redacted")
    );
    assert_eq!(
        super::parse_api_key("OPENAI_API_KEY=test-key-redacted").as_deref(),
        Some("test-key-redacted")
    );
    assert_eq!(super::unquote("'single-quoted'"), "single-quoted");
    assert!(super::parse_api_key("not a key").is_none());

    let root = crate::self_tests::boundaries::support::temp_root("openai-policy-invalid-env");
    let path = root.join(".codex-worktree/env.sh");
    std::fs::create_dir_all(path.parent().unwrap()).expect("env dir");
    std::fs::write(&path, [0xff]).expect("invalid utf8");
    let state = super::inspect_env_file(&path);
    assert!(state.exists);
    assert!(state.malformed);
    assert!(!super::destination_is_ignored(
        &root,
        std::path::Path::new("missing/env.sh")
    ));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn api_key_reader_errors_and_missing_key_are_typed() {
    let root = crate::self_tests::openai::prepare_root(
        "openai-policy-api-key-reader",
        &["docs/openai-key-policy.json"],
    );
    let state = super::load(
        &root,
        std::path::Path::new(crate::cli::openai::config::DEFAULT_POLICY),
    );
    assert_eq!(
        super::api_key_from_state(&root, &state, |_| Err(std::io::Error::other("read failed")))
            .unwrap_err(),
        "openai_key_read_failed"
    );
    assert_eq!(
        super::api_key_from_state(&root, &state, |_| Ok("not a key".to_string())).unwrap_err(),
        "openai_key_not_found"
    );
    assert_eq!(
        super::api_key(
            &root,
            std::path::Path::new(crate::cli::openai::config::DEFAULT_POLICY),
        )
        .expect("api key"),
        "test-key-redacted"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
