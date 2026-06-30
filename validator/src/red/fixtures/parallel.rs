use crate::json_boundary;
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
    let shared = Shared::new(&input);
    let mut parallel_rows = Vec::new();
    let mut serial_rows = Vec::new();
    for row in input.items {
        if row_allows_parallel(input.root, row) {
            parallel_rows.push(row.clone());
        } else {
            serial_rows.push(row.clone());
        }
    }
    let mut rows = BTreeMap::new();
    let mut metrics = Vec::new();
    if !parallel_rows.is_empty() {
        let scheduled = crate::scheduler::run_ordered(
            input.scheduler,
            TaskClass::PureReadParallel,
            tasks(shared.clone(), parallel_rows),
        );
        metrics.push(scheduled.metrics);
        rows.extend(scheduled.values);
    }
    if !serial_rows.is_empty() {
        let scheduled = crate::scheduler::run_ordered(
            input.scheduler,
            TaskClass::SharedAuthorityWriteSerial,
            tasks(shared, serial_rows),
        );
        metrics.push(scheduled.metrics);
        rows.extend(scheduled.values);
    }
    super::FixtureResults {
        rows,
        scheduler_metrics: metrics,
    }
}

#[derive(Clone)]
struct Shared {
    root: Arc<PathBuf>,
    store: Arc<SchemaStore>,
    validator_digests: Arc<BTreeMap<String, String>>,
    target_digest: Arc<String>,
    runtime_digest_cache: Arc<BTreeMap<String, String>>,
    runtime_red_fixture_ids: Arc<Vec<String>>,
    runtime_red_fixtures: Arc<Value>,
    runtime_input_digests: Arc<Vec<Value>>,
}

impl Shared {
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

fn tasks(shared: Shared, rows: Vec<Value>) -> Vec<RowTask> {
    rows.into_iter()
        .map(|row| {
            let shared = shared.clone();
            Box::new(move || evaluate_row(shared, row)) as RowTask
        })
        .collect()
}

fn evaluate_row(shared: Shared, row: Value) -> (String, Value) {
    let row_id = row
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("invalid-row")
        .to_string();
    let mut runtime_digest_cache = (*shared.runtime_digest_cache).clone();
    let mut semantic_cache = crate::claim_semantics::SemanticCache::default();
    let result = super::result_for_row(
        &shared.root,
        &shared.store,
        &shared.validator_digests,
        &row,
        &shared.target_digest,
        &mut runtime_digest_cache,
        &shared.runtime_red_fixture_ids,
        &shared.runtime_red_fixtures,
        &shared.runtime_input_digests,
        &mut semantic_cache,
    );
    (row_id, result)
}

fn row_allows_parallel(root: &Path, row: &Value) -> bool {
    let packet_rel = row.get("packet_path").and_then(Value::as_str).unwrap_or("");
    if crate::package::inventory::package_path_error(root, packet_rel).is_some() {
        return true;
    }
    let packet_path = match crate::package::inventory::resolve(root, packet_rel) {
        Ok(path) => path,
        Err(_) => return true,
    };
    let Ok(packet) = json_boundary::read_json(&packet_path) else {
        return true;
    };
    !packet
        .get("filesystem_fixtures")
        .and_then(Value::as_array)
        .is_some_and(|fixtures| !fixtures.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{EvaluateInput, evaluate, row_allows_parallel};
    use crate::scheduler::SchedulerConfig;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::fs;

    #[test]
    fn filesystem_fixture_rows_stay_serial() {
        let root = crate::self_tests::boundaries::support::temp_root("red-fixture-scheduler");
        fs::create_dir_all(root.join("fixtures/red")).expect("fixtures");
        crate::json_boundary::write_json(
            &root.join("fixtures/red/no-filesystem.json"),
            &json!({"expected_failure": {"code": "x"}}),
        )
        .expect("parallel packet");
        crate::json_boundary::write_json(
            &root.join("fixtures/red/filesystem.json"),
            &json!({
                "expected_failure": {"code": "x"},
                "filesystem_fixtures": [{"kind": "file", "path": "tmp/red.txt", "content": "x"}]
            }),
        )
        .expect("serial packet");
        assert!(row_allows_parallel(
            &root,
            &json!({"packet_path": "fixtures/red/no-filesystem.json"})
        ));
        assert!(!row_allows_parallel(
            &root,
            &json!({"packet_path": "fixtures/red/filesystem.json"})
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn evaluator_records_parallel_and_serial_scheduler_metrics() {
        let root = crate::self_tests::boundaries::support::temp_root("red-fixture-metrics");
        fs::create_dir_all(root.join("fixtures/red")).expect("fixtures");
        crate::json_boundary::write_json(
            &root.join("fixtures/red/filesystem.json"),
            &json!({
                "expected_failure": {"code": "red_fixture_base_path_not_allowed"},
                "filesystem_fixtures": [{"kind": "file", "path": "tmp/red.txt", "content": "x"}]
            }),
        )
        .expect("serial packet");
        let store = crate::schema_catalog::load(&root);
        let runtime_cache = BTreeMap::new();
        let runtime_ids = Vec::new();
        let runtime_rows = json!({});
        let runtime_inputs = Vec::new();
        let results = evaluate(EvaluateInput {
            root: &root,
            store: &store,
            validator_digests: &BTreeMap::new(),
            items: &[
                json!({"id": "missing-packet", "packet_path": "fixtures/red/missing.json"}),
                json!({"id": "filesystem-packet", "packet_path": "fixtures/red/filesystem.json"}),
            ],
            target_digest: "sha256:test",
            runtime_digest_cache: &runtime_cache,
            runtime_red_fixture_ids: &runtime_ids,
            runtime_red_fixtures: &runtime_rows,
            runtime_input_digests: &runtime_inputs,
            scheduler: SchedulerConfig::from_jobs(Some(4)).expect("scheduler"),
        });
        assert_eq!(results.rows.len(), 2);
        assert!(
            results
                .scheduler_metrics
                .iter()
                .any(|metric| metric.task_class == "pure_read_parallel" && metric.task_count == 1)
        );
        assert!(results.scheduler_metrics.iter().any(|metric| {
            metric.task_class == "shared_authority_write_serial"
                && metric.worker_count == 1
                && metric.task_count == 1
        }));
        fs::remove_dir_all(root).expect("cleanup");
    }
}
