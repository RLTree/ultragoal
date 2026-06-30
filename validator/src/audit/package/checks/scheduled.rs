use super::{inventory_checks, push};
use crate::scheduler::{SchedulerConfig, TaskClass};
use crate::schema_catalog::SchemaStore;
use crate::target_fixtures;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

type Failures = BTreeMap<String, Vec<String>>;
type PackageCheckTask = Box<dyn FnOnce() -> Failures + Send>;

pub(super) fn package_checks(
    root: &Path,
    store: &SchemaStore,
    check_ids: &[String],
    validator_artifacts: &[Value],
    scheduler: SchedulerConfig,
    failures: &mut Failures,
) -> Vec<crate::scheduler::Metrics> {
    let tasks = package_check_tasks(
        Arc::new(root.to_path_buf()),
        Arc::new(store.clone()),
        Arc::new(check_ids.to_vec()),
        Arc::new(validator_artifacts.to_vec()),
    );
    let scheduled = crate::scheduler::run_ordered(scheduler, TaskClass::PureReadParallel, tasks);
    for result in scheduled.values {
        merge_failures(failures, result);
    }
    vec![scheduled.metrics]
}

fn package_check_tasks(
    root: Arc<PathBuf>,
    store: Arc<SchemaStore>,
    check_ids: Arc<Vec<String>>,
    validator_artifacts: Arc<Vec<Value>>,
) -> Vec<PackageCheckTask> {
    vec![
        task({
            let root = Arc::clone(&root);
            move |out| inventory_checks(root.as_path(), out)
        }),
        task({
            let root = Arc::clone(&root);
            let store = Arc::clone(&store);
            move |out| crate::audit::red::catalog::check(root.as_path(), &store, out)
        }),
        task({
            let root = Arc::clone(&root);
            let store = Arc::clone(&store);
            let check_ids = Arc::clone(&check_ids);
            move |out| {
                crate::audit::package::text_checks::run(&root, &store, &check_ids, out);
            }
        }),
        task({
            let root = Arc::clone(&root);
            move |out| crate::audit::review_history::check(root.as_path(), out)
        }),
        task({
            let root = Arc::clone(&root);
            move |out| {
                out.entry("research-source-authority-article-to-law-integration".to_string())
                    .or_default()
                    .extend(crate::audit::research::failures(root.as_path()));
            }
        }),
        task({
            let root = Arc::clone(&root);
            move |out| {
                out.entry("harness-improvement-loop-trace-feedback-eval-codex-handoff".to_string())
                    .or_default()
                    .extend(crate::audit::improvement_loop::package_failures(
                        root.as_path(),
                    ));
            }
        }),
        task({
            let root = Arc::clone(&root);
            move |out| {
                for failure in crate::review::round::fixture_failures(root.as_path()) {
                    push(out, "validator-execution-provenance", failure);
                }
                for failure in crate::review::materiality::fixture_failures(root.as_path()) {
                    push(out, "material-review-scope-gate", failure);
                }
            }
        }),
        task({
            let root = Arc::clone(&root);
            let validator_artifacts = Arc::clone(&validator_artifacts);
            move |out| {
                out.entry("target-repo-audit-capability".to_string())
                    .or_default()
                    .extend(target_fixtures::target_capability_failures(
                        root.as_path(),
                        &validator_artifacts,
                    ));
            }
        }),
    ]
}

fn task(f: impl FnOnce(&mut Failures) + Send + 'static) -> PackageCheckTask {
    Box::new(move || {
        let mut out = BTreeMap::new();
        f(&mut out);
        out
    })
}

fn merge_failures(failures: &mut Failures, source: Failures) {
    for (check, items) in source {
        failures.entry(check).or_default().extend(items);
    }
}
