#[test]
fn noncanonical_jobs_do_not_mint_routine_cache_authority() {
    let root = super::package_root(
        "line-caps-noncanonical-jobs",
        &[("validator/src/lib.rs", "pub fn ok() {}\n".to_string())],
    );
    let code = crate::command_run::run_with_exit_code(super::args(
        root.clone(),
        &["line-caps", "check", "--strict", "--jobs", "2"],
    ))
    .expect("line caps pass");
    assert_eq!(code, 0);
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/line-cap-check.json"),
    )
    .expect("receipt");
    assert!(receipt.get("cache_records").is_none());
    std::fs::remove_dir_all(root).expect("cleanup noncanonical jobs");
}
