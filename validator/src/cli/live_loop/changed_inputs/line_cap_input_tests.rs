use super::path_affects_surface;

#[test]
fn line_cap_inputs_match_the_source_line_count_law() {
    for path in [
        "validator/src/cli/live_loop/mod.rs",
        ".harness/bootstrap.sh",
        ".harness/coverage-command",
        "scripts/check",
        "scripts/check-agent-standards",
        "scripts/check-coverage-fast",
        "scripts/check-coverage-full",
    ] {
        assert!(path_affects_surface(path, "line_caps_check"), "{path}");
    }
    assert!(!path_affects_surface(
        "schemas/example.json",
        "line_caps_check"
    ));
}
