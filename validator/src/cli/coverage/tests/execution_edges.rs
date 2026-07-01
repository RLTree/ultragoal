use super::*;
use serde_json::json;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[test]
fn coverage_parse_and_scheduler_edges_are_typed() {
    let default = parse(&["coverage".into(), "prove".into()])
        .expect("parse")
        .expect("command");
    assert_eq!(default.receipt, PathBuf::from(COVERAGE_RECEIPT_REL));
    assert!(default.jobs.is_none());
    assert!(!default.validate_existing);

    assert!(
        parse(&["coverage".into(), "prove".into(), "--receipt".into()])
            .expect_err("missing receipt")
            .contains("missing value for --receipt")
    );
    assert!(
        parse(&["coverage".into(), "prove".into(), "--jobs".into()])
            .expect_err("missing jobs")
            .contains("missing value for --jobs")
    );
    assert!(
        parse(&[
            "coverage".into(),
            "prove".into(),
            "--jobs".into(),
            "fast".into()
        ])
        .expect_err("invalid jobs")
        .contains("invalid numeric value for --jobs")
    );

    let root = crate::self_tests::boundaries::support::temp_root("coverage-jobs-zero");
    super::write_coverage_root(&root, 100.0, json!([]));
    let command = CoverageCommand {
        receipt: PathBuf::from(COVERAGE_RECEIPT_REL),
        jobs: Some(0),
        validate_existing: true,
    };
    assert!(
        run(&root, &command)
            .expect_err("bounded scheduler")
            .contains("scheduler jobs must be at least 1")
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn coverage_dispatch_and_receipt_write_failures_are_observable() {
    let root = crate::self_tests::boundaries::support::temp_root("coverage-dispatch");
    super::write_coverage_root(&root, 100.0, json!([]));
    let raw = [
        "coverage",
        "prove",
        "--validate-existing",
        "--receipt",
        COVERAGE_RECEIPT_REL,
    ]
    .iter()
    .map(|item| item.to_string())
    .collect::<Vec<_>>();
    let command = crate::parse_command(&raw).expect("parse coverage command");
    let code = crate::command_run::run_with_exit_code(crate::Args {
        root: root.clone(),
        command,
    })
    .expect("dispatch");
    assert_eq!(code, 0);
    assert!(root.join(OBSERVABILITY_RECEIPT_REL).is_file());

    let write_error_root =
        crate::self_tests::boundaries::support::temp_root("coverage-write-error");
    super::write_coverage_root(&write_error_root, 100.0, json!([]));
    fs::create_dir_all(write_error_root.join("validation_artifacts")).expect("artifact dir");
    fs::write(
        write_error_root.join("validation_artifacts/observability"),
        "not a directory",
    )
    .expect("blocking file");
    let command = CoverageCommand {
        receipt: PathBuf::from(COVERAGE_RECEIPT_REL),
        jobs: Some(1),
        validate_existing: true,
    };
    let err = run(&write_error_root, &command).expect_err("receipt write blocked");
    assert!(
        err.contains("validation_artifacts/observability")
            || err.contains("Not a directory")
            || err.contains("not a directory"),
        "{err}"
    );
    fs::remove_dir_all(root).expect("cleanup dispatch");
    fs::remove_dir_all(write_error_root).expect("cleanup write error");
}

#[test]
fn coverage_authoritative_script_and_package_mutation_are_observed() {
    let root = crate::self_tests::boundaries::support::temp_root("coverage-authoritative");
    super::write_coverage_root(&root, 100.0, json!([]));
    fs::create_dir_all(root.join("scripts")).expect("scripts");
    fs::write(root.join("scripts/check-coverage-full"), "exit 0\n").expect("script");
    let command = CoverageCommand {
        receipt: PathBuf::from(COVERAGE_RECEIPT_REL),
        jobs: Some(1),
        validate_existing: false,
    };
    assert_eq!(run(&root, &command).expect("authoritative pass"), 0);

    let mutation_root =
        crate::self_tests::boundaries::support::temp_root("coverage-package-mutation");
    super::write_coverage_root(&mutation_root, 100.0, json!([]));
    let mutation_command = CoverageCommand {
        receipt: PathBuf::from(COVERAGE_RECEIPT_REL),
        jobs: Some(1),
        validate_existing: false,
    };
    assert_eq!(
        run_with_executor(&mutation_root, &mutation_command, mutating_executor).expect("run"),
        1
    );
    let receipt = crate::json_boundary::read_json(&mutation_root.join(OBSERVABILITY_RECEIPT_REL))
        .expect("mutation receipt");
    assert!(
        receipt["why_failed"]
            .as_str()
            .unwrap()
            .contains("coverage_command_mutated_package_digest")
    );
    fs::remove_dir_all(root).expect("cleanup authoritative");
    fs::remove_dir_all(mutation_root).expect("cleanup mutation");
}

#[test]
fn coverage_diagnostic_fallbacks_are_actionable_and_bounded() {
    let spawn = execution_from_output(Err(io::Error::new(io::ErrorKind::NotFound, "missing bash")));
    assert_eq!(spawn.code, 2);
    assert!(spawn.stderr.contains("coverage_command_spawn_failed"));

    let plain = CoverageExecution {
        code: 2,
        stdout: "plain failure\n".to_string(),
        stderr: "info: preface\nwarning: detail\n".to_string(),
        cache_mode: "coverage_authoritative_no_cache",
    };
    assert_eq!(first_diagnostic(&plain), "plain failure");
    assert!(is_noise_diagnostic("warning: detail"));

    let noise_only = CoverageExecution {
        code: 2,
        stdout: String::new(),
        stderr: "info: only noise\n".to_string(),
        cache_mode: "coverage_authoritative_no_cache",
    };
    assert_eq!(first_diagnostic(&noise_only), "info: only noise");

    let empty = CoverageExecution {
        code: 2,
        stdout: String::new(),
        stderr: String::new(),
        cache_mode: "coverage_authoritative_no_cache",
    };
    assert_eq!(first_diagnostic(&empty), "coverage command exited nonzero");
}

fn mutating_executor(root: &Path, _receipt: &Path) -> CoverageExecution {
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["src/lib.rs"]}),
    )
    .expect("mutate manifest");
    CoverageExecution::validate_existing()
}
