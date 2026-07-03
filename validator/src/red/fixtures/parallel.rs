use crate::json_boundary;
use crate::red::fixture::row::invalid_row;
use crate::scheduler::{SchedulerConfig, TaskClass};
use crate::schema_catalog::SchemaStore;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(super) struct EvaluateInput<'a> {
    pub(super) root: &'a Path,
    pub(super) store: &'a SchemaStore,
    pub(super) validator_digests: &'a BTreeMap<String, String>,
    pub(super) items: &'a [Value],
    pub(super) target_digest: &'a str,
    pub(super) runtime_digest_cache: &'a BTreeMap<String, String>,
    pub(super) runtime_red_fixture_ids: &'a [String],
    pub(super) runtime_red_fixtures: &'a Value,
    pub(super) runtime_input_digests: &'a [Value],
    pub(super) scheduler: SchedulerConfig,
}

pub(super) fn evaluate(input: EvaluateInput<'_>) -> super::FixtureResults {
    let runtime = FixtureRuntime::new(&input);
    let mut pure_rows = Vec::new();
    let mut isolated_rows = Vec::new();
    for row in input.items {
        match row_mode(input.root, row) {
            RowMode::PureRead => pure_rows.push(row.clone()),
            RowMode::IsolatedTempWrite => isolated_rows.push(row.clone()),
        }
    }
    let mut rows = BTreeMap::new();
    let mut metrics = Vec::new();
    if !pure_rows.is_empty() {
        let scheduled = crate::scheduler::run_ordered(
            input.scheduler,
            TaskClass::PureReadParallel,
            tasks(runtime.clone(), pure_rows, RowMode::PureRead),
        );
        metrics.push(scheduled.metrics);
        rows.extend(scheduled.values);
    }
    if !isolated_rows.is_empty() {
        let scheduled = crate::scheduler::run_ordered(
            input.scheduler,
            TaskClass::IsolatedTempWriteParallel,
            tasks(runtime, isolated_rows, RowMode::IsolatedTempWrite),
        );
        metrics.push(scheduled.metrics);
        rows.extend(scheduled.values);
    }
    super::FixtureResults {
        rows,
        scheduler_metrics: metrics,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RowMode {
    PureRead,
    IsolatedTempWrite,
}

#[derive(Clone)]
struct FixtureRuntime {
    root: Arc<PathBuf>,
    store: Arc<SchemaStore>,
    validator_digests: Arc<BTreeMap<String, String>>,
    target_digest: Arc<String>,
    runtime_digest_cache: Arc<BTreeMap<String, String>>,
    runtime_red_fixture_ids: Arc<Vec<String>>,
    runtime_red_fixtures: Arc<Value>,
    runtime_input_digests: Arc<Vec<Value>>,
}

impl FixtureRuntime {
    fn new(input: &EvaluateInput<'_>) -> Self {
        Self {
            root: Arc::new(input.root.to_path_buf()),
            store: Arc::new(input.store.clone()),
            validator_digests: Arc::new(input.validator_digests.clone()),
            target_digest: Arc::new(input.target_digest.to_string()),
            runtime_digest_cache: Arc::new(input.runtime_digest_cache.clone()),
            runtime_red_fixture_ids: Arc::new(input.runtime_red_fixture_ids.to_vec()),
            runtime_red_fixtures: Arc::new(input.runtime_red_fixtures.clone()),
            runtime_input_digests: Arc::new(input.runtime_input_digests.to_vec()),
        }
    }
}

type RowTask = Box<dyn FnOnce() -> (String, Value) + Send>;

fn tasks(runtime: FixtureRuntime, rows: Vec<Value>, mode: RowMode) -> Vec<RowTask> {
    rows.into_iter()
        .map(|row| {
            let runtime = runtime.clone();
            Box::new(move || evaluate_row(runtime, row, mode)) as RowTask
        })
        .collect()
}

fn evaluate_row(runtime: FixtureRuntime, row: Value, mode: RowMode) -> (String, Value) {
    let row_id = row
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("invalid-row")
        .to_string();
    let packet_rel = row.get("packet_path").and_then(Value::as_str).unwrap_or("");
    let result = match mode {
        RowMode::PureRead => evaluate_row_at_root(&runtime, &row, &runtime.root),
        RowMode::IsolatedTempWrite => super::isolation::with_isolated_root(
            &runtime.root,
            &row_id,
            &[packet_rel],
            |isolated_root| evaluate_row_at_root(&runtime, &row, isolated_root),
        )
        .unwrap_or_else(|error| invalid_row(&row, &error)),
    };
    (row_id, result)
}

fn evaluate_row_at_root(runtime: &FixtureRuntime, row: &Value, root: &Path) -> Value {
    let mut runtime_digest_cache = (*runtime.runtime_digest_cache).clone();
    let mut semantic_cache = crate::claim_semantics::SemanticCache::default();
    super::result_for_row(
        root,
        &runtime.store,
        &runtime.validator_digests,
        row,
        &runtime.target_digest,
        &mut runtime_digest_cache,
        &runtime.runtime_red_fixture_ids,
        &runtime.runtime_red_fixtures,
        &runtime.runtime_input_digests,
        &mut semantic_cache,
    )
}

fn row_mode(root: &Path, row: &Value) -> RowMode {
    let packet_rel = row.get("packet_path").and_then(Value::as_str).unwrap_or("");
    if crate::package::inventory::package_path_error(root, packet_rel).is_some() {
        return RowMode::PureRead;
    }
    let packet_path = root.join(packet_rel);
    let Ok(packet) = json_boundary::read_json(&packet_path) else {
        return RowMode::PureRead;
    };
    if super::isolation::has_filesystem_fixtures(&packet) {
        RowMode::IsolatedTempWrite
    } else {
        RowMode::PureRead
    }
}

#[cfg(test)]
mod tests {
    use super::{EvaluateInput, evaluate};
    use crate::scheduler::SchedulerConfig;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::fs;

    #[test]
    fn evaluator_records_parallel_and_isolated_scheduler_metrics() {
        let root = crate::self_tests::boundaries::support::temp_root("red-fixture-metrics");
        fs::create_dir_all(root.join("fixtures/red")).expect("fixtures");
        fs::create_dir_all(root.join("tmp")).expect("tmp");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({
                "resources":[
                    "fixtures/red/filesystem-one.json",
                    "fixtures/red/filesystem-two.json"
                ]
            }),
        )
        .expect("manifest");
        for (name, path) in [
            ("filesystem-one.json", "tmp/red-one.txt"),
            ("filesystem-two.json", "tmp/red-two.txt"),
        ] {
            crate::json_boundary::write_json(
                &root.join("fixtures/red").join(name),
                &json!({
                    "expected_failure": {"error": "invalid_base_fixture_path"},
                    "filesystem_fixtures": [{"kind": "file", "path": path, "contents": "x"}]
                }),
            )
            .expect("isolated packet");
        }
        let store = crate::schema_catalog::load(&root);
        let runtime_cache = BTreeMap::new();
        let runtime_rows = json!({});
        let results = evaluate(EvaluateInput {
            root: &root,
            store: &store,
            validator_digests: &BTreeMap::new(),
            items: &[
                json!({"id": "missing-packet", "packet_path": "fixtures/red/missing.json"}),
                json!({"id": "filesystem-one", "packet_path": "fixtures/red/filesystem-one.json"}),
                json!({"id": "filesystem-two", "packet_path": "fixtures/red/filesystem-two.json"}),
            ],
            target_digest: "sha256:test",
            runtime_digest_cache: &runtime_cache,
            runtime_red_fixture_ids: &[],
            runtime_red_fixtures: &runtime_rows,
            runtime_input_digests: &[],
            scheduler: SchedulerConfig::from_jobs(Some(4)).expect("scheduler"),
        });
        assert_eq!(results.rows.len(), 3);
        assert!(
            results
                .scheduler_metrics
                .iter()
                .any(|metric| metric.task_class == "pure_read_parallel" && metric.task_count == 1)
        );
        assert!(results.scheduler_metrics.iter().any(|metric| {
            metric.task_class == "isolated_temp_write_parallel"
                && metric.worker_count > 1
                && metric.task_count == 2
        }));
        assert!(!root.join("tmp/red-one.txt").exists());
        assert!(!root.join("tmp/red-two.txt").exists());
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({"resources":["fixtures/red/filesystem-one.json","../escape.json"]}),
        )
        .expect("invalid manifest path");
        let failed_rows =
            [json!({"id": "filesystem-one", "packet_path": "fixtures/red/filesystem-one.json"})];
        let failed = evaluate(EvaluateInput {
            root: &root,
            store: &store,
            validator_digests: &BTreeMap::new(),
            items: &failed_rows,
            target_digest: "sha256:test",
            runtime_digest_cache: &runtime_cache,
            runtime_red_fixture_ids: &[],
            runtime_red_fixtures: &runtime_rows,
            runtime_input_digests: &[],
            scheduler: SchedulerConfig::from_jobs(Some(2)).expect("scheduler"),
        });
        let observed = failed.rows["filesystem-one"]["observed_error"]
            .as_str()
            .unwrap();
        assert!(observed.contains("red_fixture_isolated_root_path_invalid"));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
