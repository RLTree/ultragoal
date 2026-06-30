use super::super::{prepare_root, write_call_receipt};

#[test]
fn openai_output_defaults_and_error_boundaries_are_typed() {
    let missing_root = std::env::temp_dir().join(format!(
        "ultragoal-openai-output-missing-root-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&missing_root);
    let default = parse_openai(&["openai", "output", "prove"]);
    assert!(crate::cli::openai::run(&missing_root, &default).is_err());

    let root = prepare_root("openai-output-error-boundaries");
    write_call_receipt(&root);
    let bad_default =
        crate::cli::openai::build_receipt(&root, &default).expect("default output receipt");
    assert_eq!(bad_default["status"], "pass");
    let bad_parser = parse_openai(&["openai", "output", "prove", "--parser-schema-id", ""]);
    let bad_parser_receipt =
        crate::cli::openai::build_receipt(&root, &bad_parser).expect("bad parser receipt");
    assert!(
        bad_parser_receipt["failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure.as_str() == Some("openai_output_parser_schema_missing"))
    );

    std::fs::remove_dir_all(root.join("validation_artifacts/openai")).expect("remove output dir");
    std::fs::write(root.join("validation_artifacts/openai"), "not a directory")
        .expect("spool blocker");
    let write_error = parse_openai(&[
        "openai",
        "output",
        "prove",
        "--parsed-output-digest",
        &crate::self_tests::boundaries::support::sha('e'),
        "--receipt",
        "validation_artifacts/openai/write-error.json",
    ]);
    assert!(crate::cli::openai::run(&root, &write_error).is_err());
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn parse_openai(args: &[&str]) -> crate::cli::openai::OpenAiCommand {
    crate::cli::openai::parse(
        &args
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>(),
    )
    .expect("parse openai")
    .expect("openai command")
}
