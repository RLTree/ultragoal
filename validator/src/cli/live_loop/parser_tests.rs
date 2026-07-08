use super::{LiveLoopAction, parse};

#[test]
fn parser_accepts_auto_jobs_and_rejects_invalid_jobs() {
    let command = parse(
        &[
            "loop",
            "run",
            "--tier",
            "hot",
            "--cache-mode",
            "verified-local",
            "--jobs",
            "8",
            "--receipt",
            "validation_artifacts/observability/custom-loop.json",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>(),
    )
    .expect("parse")
    .expect("loop command");
    assert_eq!(command.action, LiveLoopAction::Run);
    assert_eq!(command.tier, "hot");
    assert_eq!(command.cache_mode, "verified-local");
    assert_eq!(command.jobs, Some(8));
    assert_eq!(
        command.receipt,
        std::path::PathBuf::from("validation_artifacts/observability/custom-loop.json")
    );

    let err = parse(
        &["loop", "run", "--jobs", "many"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("invalid jobs");
    assert!(err.contains("invalid --jobs value"));

    let measure = parse(
        &[
            "loop",
            "measure",
            "--node",
            "fmt_check",
            "--jobs",
            "2",
            "--receipt",
            "validation_artifacts/observability/custom-node-timing.json",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>(),
    )
    .expect("measure parse")
    .expect("measure command");
    assert_eq!(measure.action, LiveLoopAction::Measure);
    assert_eq!(measure.node_id.as_deref(), Some("fmt_check"));
    assert_eq!(measure.jobs, Some(2));
    assert_eq!(
        measure.receipt,
        std::path::PathBuf::from("validation_artifacts/observability/custom-node-timing.json")
    );
    assert!(!measure.measure_all);

    let measure_all = parse(
        &["loop", "measure", "--all", "--jobs", "auto"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect("measure all parse")
    .expect("measure all command");
    assert_eq!(measure_all.action, LiveLoopAction::Measure);
    assert_eq!(measure_all.node_id, None);
    assert_eq!(measure_all.jobs, None);
    assert!(measure_all.measure_all);

    let err = parse(
        &["loop", "measure", "--node", "fmt_check", "--jobs", "many"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("invalid measure jobs");
    assert!(err.contains("invalid --jobs value"));

    let err = parse(
        &["loop", "measure", "--all", "--node", "fmt_check"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("ambiguous measure target");
    assert!(err.contains("either --all or --node"));

    let err = parse(
        &["loop", "measure"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("missing node");
    assert!(err.contains("loop measure requires --node <id> or --all"));

    let format = parse(
        &["loop", "format", "check", "--changed-rust"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect("format check parse")
    .expect("format check command");
    assert_eq!(format.action, LiveLoopAction::FormatCheck);

    let err = parse(
        &["loop", "format", "check"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("missing changed-rust");
    assert!(err.contains("loop format check requires --changed-rust"));
}
