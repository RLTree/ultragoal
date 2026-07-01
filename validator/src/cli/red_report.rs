use std::path::{Path, PathBuf};

const DEFAULT_REPORT: &str = "validation_artifacts/ultragoal-audit/red-fixture-report.json";

#[derive(Debug)]
pub(crate) struct RedReportCommand {
    pub(crate) report: PathBuf,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<RedReportCommand>, String> {
    let args = match raw {
        [first, second, third, rest @ ..]
            if first == "red" && second == "fixture" && third == "report" =>
        {
            rest
        }
        [first, rest @ ..] if first == "red-fixture-report" => rest,
        _ => return Ok(None),
    };
    let report = opt_report(args)?;
    Ok(Some(RedReportCommand {
        report: report.unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT)),
    }))
}

pub(crate) fn run(root: &Path, command: &RedReportCommand) -> Result<i32, String> {
    crate::cli::audit::run_red_fixture_report(root, &command.report)
}

fn opt_report(args: &[String]) -> Result<Option<PathBuf>, String> {
    let mut report = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--report" | "--red-report" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("missing value for {}", args[index]))?;
                report = Some(PathBuf::from(value));
                index += 2;
            }
            other => return Err(format!("unknown red fixture report argument: {other}")),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn red_fixture_report_parse_supports_default_and_alias() {
        let command = parse(&["red".into(), "fixture".into(), "report".into()])
            .expect("parse")
            .expect("command");
        assert_eq!(command.report, PathBuf::from(DEFAULT_REPORT));

        let alias = parse(&[
            "red-fixture-report".into(),
            "--red-report".into(),
            "custom.json".into(),
        ])
        .expect("alias parse")
        .expect("alias command");
        assert_eq!(alias.report, PathBuf::from("custom.json"));

        assert!(
            parse(&["source".into(), "audit".into()])
                .expect("other")
                .is_none()
        );
        assert!(
            parse(&[
                "red".into(),
                "fixture".into(),
                "report".into(),
                "--report".into()
            ])
            .expect_err("missing report")
            .contains("missing value for --report")
        );
        assert!(
            parse(&["red-fixture-report".into(), "--bogus".into()])
                .expect_err("unknown")
                .contains("unknown red fixture report argument")
        );
    }

    #[test]
    fn red_fixture_report_command_writes_dedicated_observability() {
        let root = crate::self_tests::boundaries::support::temp_root("red-report-command");
        let audit_dir = root.join("validation_artifacts/ultragoal-audit");
        fs::create_dir_all(&audit_dir).expect("audit dir");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({"resources":[]}),
        )
        .expect("manifest");
        let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
        let report = audit_dir.join("red-fixture-report.json");
        crate::json_boundary::write_json(
            &report,
            &json!({
                "status":"pass",
                "target_revision":{"kind":"package_digest","value":candidate},
                "red_fixtures":{"red-one":{"status":"pass"}}
            }),
        )
        .expect("pass report");

        let command = RedReportCommand {
            report: report.clone(),
        };
        assert_eq!(run(&root, &command).expect("pass run"), 0);
        let receipt_path = root.join("validation_artifacts/observability/red-fixture-report.json");
        let pass = crate::json_boundary::read_json(&receipt_path).expect("pass receipt");
        assert_eq!(pass["status"], "pass");
        assert_eq!(pass["event"]["command"], "ultragoal red");
        assert_eq!(pass["event"]["subcommand"], "fixture report");
        assert_eq!(pass["operation"], "red_fixture.report");
        assert_eq!(pass["event"]["cache_mode"], "red_report_standalone_read");
        assert_eq!(
            pass["event"]["resource_measurement_status"],
            "standalone_red_report_wall_time_only"
        );
        assert_eq!(
            pass["event"]["repair_anchor_before"],
            "red_fixture_report_command_start"
        );
        let pass_stdout = crate::cli::audit::red_report_stdout_contract_for_test(&pass);
        assert_eq!(pass_stdout.len(), 1);
        assert!(pass_stdout[0].contains("operation=red_fixture.report"));

        let dispatch = crate::parse_command(&[
            "red".to_string(),
            "fixture".to_string(),
            "report".to_string(),
            "--report".to_string(),
            report.display().to_string(),
        ])
        .expect("parse dispatch");
        let code = crate::command_run::run_with_exit_code(crate::Args {
            root: root.clone(),
            command: dispatch,
        })
        .expect("dispatch run");
        assert_eq!(code, 0);

        crate::json_boundary::write_json(
            &report,
            &json!({"status":"fail","red_fixtures":{"bad":{"status":"fail"}}}),
        )
        .expect("fail report");
        assert_eq!(run(&root, &command).expect("fail run"), 1);
        let fail = crate::json_boundary::read_json(&receipt_path).expect("fail receipt");
        assert_eq!(fail["status"], "fail");
        assert_eq!(fail["event"]["failure_class"], "red_fixture_report_failure");
        assert!(
            fail["why_failed"]
                .as_str()
                .unwrap_or_default()
                .contains("bad")
        );
        let fail_stdout = crate::cli::audit::red_report_stdout_contract_for_test(&fail);
        assert!(
            fail_stdout
                .iter()
                .any(|line| line.contains("failed_check=red-fixture-report-observability-binding"))
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn red_fixture_report_command_propagates_observability_write_errors() {
        let root = crate::self_tests::boundaries::support::temp_root("red-report-no-candidate");
        let command = RedReportCommand {
            report: PathBuf::from(DEFAULT_REPORT),
        };
        let err = run(&root, &command).expect_err("missing candidate blocks observation");
        assert!(err.contains("plugin-manifest-draft.json"), "{err}");
        let _ = fs::remove_dir_all(root);
    }
}
