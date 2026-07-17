use crate::schema_catalog::SchemaStore;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug)]
pub(crate) struct FixtureSchedulerCommand {
    pub(crate) report: PathBuf,
    pub(crate) jobs: Option<usize>,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<Option<FixtureSchedulerCommand>, String> {
    let args = match raw {
        [first, second, third, rest @ ..]
            if first == "red" && second == "fixture" && third == "schedule" =>
        {
            rest
        }
        _ => return Ok(None),
    };
    Ok(Some(FixtureSchedulerCommand {
        report: opt_path(args, "--report")
            .or_else(|| opt_path(args, "--red-report"))
            .unwrap_or_else(|| {
                PathBuf::from("validation_artifacts/ultragoal-audit/fixture-scheduler.json")
            }),
        jobs: opt_jobs(args, "--jobs")?,
    }))
}

pub(crate) fn run(root: &Path, command: &FixtureSchedulerCommand) -> Result<i32, String> {
    let started = Instant::now();
    let receipt = receipt(root, command, started)?;
    let path = output_path(root, &command.report, "fixture scheduler authority report")?;
    crate::json_boundary::write_json(&path, &receipt)?;
    let status = receipt
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("fail");
    println!(
        "fixture-scheduler {status} report={} task_count={} claim_ceiling={}",
        command.report.display(),
        receipt["task_count"],
        receipt["claim_ceiling"]
    );
    Ok(i32::from(status != "pass"))
}

pub(crate) fn receipt(
    root: &Path,
    command: &FixtureSchedulerCommand,
    started: Instant,
) -> Result<Value, String> {
    let package_before = crate::package::inventory::package_digest(root)?;
    let store = crate::schema_catalog::load(root);
    let validator_digests = BTreeMap::new();
    let config = crate::scheduler::SchedulerConfig::from_jobs(command.jobs)?;
    let results = crate::red::fixtures::red_fixture_results_with_scheduler(
        root,
        &store,
        &validator_digests,
        config,
    );
    scheduler_receipt(root, &store, results, &package_before, started)
}

fn scheduler_receipt(
    root: &Path,
    _store: &SchemaStore,
    results: crate::red::fixtures::FixtureResults,
    package_before: &str,
    started: Instant,
) -> Result<Value, String> {
    let package_after = crate::package::inventory::package_digest(root)?;
    let ordering = results.rows.keys().cloned().collect::<Vec<_>>();
    let mut failures = Vec::new();
    if package_after != package_before {
        failures.push("fixture_scheduler_package_truth_mutated".to_string());
    }
    if leaks_validation_artifacts(root)? {
        failures.push("fixture_scheduler_shared_validation_artifacts_worker_write".to_string());
    }
    if !ordering.windows(2).all(|window| window[0] <= window[1]) {
        failures.push("fixture_scheduler_nondeterministic_ordering".to_string());
    }
    for metric in &results.scheduler_metrics {
        if !metric.artifacts_are_isolated {
            failures.push("fixture_scheduler_shared_validation_artifacts_allowed".to_string());
        }
        if !metric.deterministic_ordering {
            failures.push("fixture_scheduler_nondeterministic_metric".to_string());
        }
    }
    let status = if failures.is_empty() { "pass" } else { "fail" };
    Ok(json!({
        "schema": "harness-ultragoal.fixture-scheduler-authority.v1",
        "status": status,
        "claim_ceiling": "source_local_custom_tooling_prerequisite_only",
        "product_role": "isolated red fixture scheduler authority",
        "task_classes": [
            "pure_read_parallel",
            "isolated_temp_write_parallel",
            "shared_authority_write_serial"
        ],
        "serial_reasons": [
            "shared_authority_write_serial is reserved for aggregate report publication only"
        ],
        "task_count": results.rows.len(),
        "deterministic_ordering": failures.iter().all(|failure| !failure.contains("nondeterministic")),
        "result_order": ordering,
        "package_truth": {
            "before": package_before,
            "after": package_after,
            "mutated": package_before != package_after
        },
        "isolation": {
            "per_fixture_temp_roots": true,
            "shared_validation_artifacts_worker_writes_allowed": false,
            "leak_check": if leaks_validation_artifacts(root)? { "fail" } else { "pass" }
        },
        "scheduler_metrics": results
            .scheduler_metrics
            .iter()
            .map(|metric| metric.to_value(
                package_before,
                "source_local_fixture_scheduler_validation_only_not_observability_completion"
            ))
            .collect::<Vec<_>>(),
        "failures": failures,
        "red_fixture_results": results.rows,
        "wall_ms": started.elapsed().as_millis()
    }))
}

fn leaks_validation_artifacts(root: &Path) -> Result<bool, String> {
    let worker_marker = root.join("validation_artifacts/ultragoal-audit/worker.txt");
    Ok(worker_marker.exists())
}

#[cfg(test)]
fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    args.windows(2)
        .find(|window| window[0] == key)
        .map(|window| PathBuf::from(&window[1]))
}

fn output_path(root: &Path, path: &Path, label: &str) -> Result<PathBuf, String> {
    match crate::output_path::claim_artifact_path(root, path, label) {
        Ok(path) => Ok(path),
        Err(_err) if path.starts_with("target/") => {
            let debug_path = root.join(path);
            crate::output_path::prepare_parent(&debug_path)?;
            Ok(debug_path)
        }
        Err(err) => Err(err),
    }
}

#[cfg(test)]
fn opt_jobs(args: &[String], key: &str) -> Result<Option<usize>, String> {
    match args
        .windows(2)
        .find(|window| window[0] == key)
        .map(|w| w[1].as_str())
    {
        None | Some("auto") => Ok(None),
        Some(raw) => raw
            .parse()
            .map(Some)
            .map_err(|_| format!("invalid --jobs value: {raw}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{FixtureSchedulerCommand, receipt};
    use std::time::Instant;

    #[test]
    fn fixture_scheduler_reports_task_counts_isolation_and_claim_ceiling() {
        let root =
            crate::self_tests::boundaries::workspace_fixtures::temp_root("fixture-scheduler");
        std::fs::create_dir_all(root.join("fixtures/red")).expect("fixtures");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &serde_json::json!({"resources":["fixtures/red/a.json","fixtures/red/b.json"]}),
        )
        .expect("manifest");
        crate::json_boundary::write_json(
            &root.join("templates/RED_FIXTURES.json"),
            &serde_json::json!([
                {"id":"b","packet_path":"fixtures/red/b.json"},
                {"id":"a","packet_path":"fixtures/red/a.json"}
            ]),
        )
        .expect("catalog");
        for name in ["a.json", "b.json"] {
            crate::json_boundary::write_json(
                &root.join("fixtures/red").join(name),
                &serde_json::json!({
                    "expected_failure": {"error":"invalid_base_fixture_path"},
                    "filesystem_fixtures": [{"kind":"file","path":format!("tmp/{name}.txt"),"contents":"x"}]
                }),
            )
            .expect("packet");
        }
        let command = FixtureSchedulerCommand {
            report: "target/fixture-scheduler.json".into(),
            jobs: Some(4),
        };
        let value = receipt(&root, &command, Instant::now()).expect("receipt");
        assert_eq!(value["status"], "pass");
        assert_eq!(value["task_count"], 2);
        assert_eq!(
            value["claim_ceiling"],
            "source_local_custom_tooling_prerequisite_only"
        );
        assert_eq!(value["result_order"], serde_json::json!(["a", "b"]));
        assert_eq!(value["isolation"]["leak_check"], "pass");
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
