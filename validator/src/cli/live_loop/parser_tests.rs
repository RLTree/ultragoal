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
            "auto",
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
    assert_eq!(command.jobs, None);

    let err = parse(
        &["loop", "run", "--jobs", "many"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("invalid jobs");
    assert!(err.contains("invalid --jobs value"));

    let measure = parse(
        &["loop", "measure", "--node", "fmt_check", "--jobs", "auto"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect("measure parse")
    .expect("measure command");
    assert_eq!(measure.action, LiveLoopAction::Measure);
    assert_eq!(measure.node_id.as_deref(), Some("fmt_check"));

    let err = parse(
        &["loop", "measure"]
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("missing node");
    assert!(err.contains("loop measure requires --node"));
}
