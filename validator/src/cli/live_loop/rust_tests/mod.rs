use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod args;
mod runner;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Eq, PartialEq)]
struct TestTarget {
    source_prefix: &'static str,
    filter: &'static str,
    reason: &'static str,
}

const TEST_TARGETS: &[TestTarget] = &[
    TestTarget {
        source_prefix: "validator/src/red/fixture/scheduler.rs",
        filter: "red::fixture::scheduler::",
        reason: "fixture scheduler changes require scheduler authority tests",
    },
    TestTarget {
        source_prefix: "validator/src/red/",
        filter: "red::",
        reason: "red fixture source changes require red module tests",
    },
    TestTarget {
        source_prefix: "validator/src/cli/red_report/",
        filter: "cli::red_report::",
        reason: "red report CLI changes require command surface tests",
    },
    TestTarget {
        source_prefix: "validator/src/cli/live_loop/rust_tests/",
        filter: "cli::live_loop::rust_tests::",
        reason: "impacted Rust test mapper changes require mapper self tests",
    },
    TestTarget {
        source_prefix: "validator/src/scheduler/",
        filter: "scheduler::",
        reason: "scheduler changes require typed scheduler tests",
    },
];

#[derive(Debug)]
pub(crate) struct ImpactedRustTestsCommand {
    pub(crate) receipt: PathBuf,
    pub(crate) jobs: Option<usize>,
    pub(crate) changed_paths: Vec<PathBuf>,
    pub(crate) run_tests: bool,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<ImpactedRustTestsCommand>, String> {
    args::parse(raw)
}

pub(crate) fn run(root: &Path, command: &ImpactedRustTestsCommand) -> Result<i32, String> {
    let started = Instant::now();
    let plan = plan(&command.changed_paths)?;
    let result = if command.run_tests && plan.status == "pass" {
        Some(runner::run_filters(root, &plan.filters)?)
    } else {
        None
    };
    let receipt = receipt(command, &plan, result.as_ref(), started);
    let receipt_path =
        args::output_path(root, &command.receipt, "impacted Rust test mapping receipt")?;
    crate::output_path::prepare_parent(&receipt_path)?;
    crate::json_boundary::write_json(&receipt_path, &receipt)?;
    println!(
        "impacted-rust-tests {} changed_count={} filter_count={} runner={} receipt={} claim_ceiling={}",
        receipt["status"],
        command.changed_paths.len(),
        plan.filters.len(),
        receipt["runner"]["kind"],
        command.receipt.display(),
        receipt["claim_ceiling"]
    );
    Ok(i32::from(receipt["status"] != "pass"))
}

#[derive(Debug)]
struct Plan {
    status: &'static str,
    filters: Vec<String>,
    mappings: Vec<Value>,
    failures: Vec<String>,
}

fn plan(changed_paths: &[PathBuf]) -> Result<Plan, String> {
    if changed_paths.is_empty() {
        return Ok(Plan {
            status: "fail",
            filters: Vec::new(),
            mappings: Vec::new(),
            failures: vec!["impacted_rust_tests_changed_path_missing".to_string()],
        });
    }
    let mut filters = BTreeSet::new();
    let mut mappings = Vec::new();
    let mut failures = Vec::new();
    for path in changed_paths {
        let rel = normalize(path);
        match TEST_TARGETS
            .iter()
            .find(|target| rel.starts_with(target.source_prefix))
        {
            Some(target) => {
                filters.insert(target.filter.to_string());
                mappings.push(json!({
                    "changed_path": rel,
                    "test_filter": target.filter,
                    "reason": target.reason,
                    "task_class": "shared_authority_write_serial",
                    "serial_reason": "one canonical cargo invocation avoids fake parallel cargo contention"
                }));
            }
            None => failures.push(format!("impacted_rust_tests_unknown_mapping:{rel}")),
        }
    }
    let filters = filters.into_iter().collect::<Vec<_>>();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    Ok(Plan {
        status,
        filters,
        mappings,
        failures,
    })
}

fn receipt(
    command: &ImpactedRustTestsCommand,
    plan: &Plan,
    run: Option<&runner::RunResult>,
    started: Instant,
) -> Value {
    let run_status = run
        .map(|result| result.status.as_str())
        .unwrap_or("not_run");
    let status = if plan.status == "pass" && run_status != "fail" {
        "pass"
    } else {
        "fail"
    };
    json!({
        "schema": "harness-ultragoal.impacted-rust-tests.v1",
        "status": status,
        "claim_ceiling": "source_local_custom_tooling_prerequisite_only",
        "changed_paths": command
            .changed_paths
            .iter()
            .map(|path| normalize(path))
            .collect::<Vec<_>>(),
        "mappings": plan.mappings,
        "failures": plan.failures,
        "runner": {
            "kind": "typed_serial_multi_filter_cargo",
            "fake_parallel_cargo_contention_rejected": true,
            "task_class": "shared_authority_write_serial",
            "requested_jobs": command.jobs.map(|jobs| jobs.to_string()).unwrap_or_else(|| "auto".to_string()),
            "effective_worker_count": 1,
            "serial_reason": "Cargo target directory and test harness output are shared authority surfaces",
            "work_unit_count": run.map(|result| result.work_unit_count).unwrap_or(0),
            "total_executed_test_count": run.map(|result| result.total_executed_test_count).unwrap_or(0),
            "result_digest": run.map(|result| result.result_digest.clone()),
            "per_filter_commands": run.map(|result| result.commands.clone()).unwrap_or_default()
        },
        "filters": plan.filters,
        "wall_ms": started.elapsed().as_millis()
    })
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
