use std::path::PathBuf;

#[test]
fn line_caps_parser_routes_to_dedicated_command() {
    let raw = ["line-caps", "check", "--strict", "--jobs", "2"];
    let command =
        crate::cli::line_caps::parse(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("line caps parse")
            .expect("line caps command");
    assert_eq!(command.jobs, Some(2));
    assert_eq!(
        command.receipt,
        PathBuf::from("validation_artifacts/observability/line-cap-check.json")
    );
    let missing = ["line-caps", "check"];
    let err = crate::parse_command(&missing.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .expect_err("strict flag required");
    assert!(err.contains("requires --strict"), "{err}");
    let err = crate::parse_command(
        &["line-caps", "check", "--strict", "--jobs"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("missing jobs value");
    assert!(err.contains("missing value for --jobs"), "{err}");
    let err = crate::parse_command(
        &["line-caps", "check", "--strict", "--bogus"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("unknown argument");
    assert!(err.contains("unknown line-caps check argument"), "{err}");
}
