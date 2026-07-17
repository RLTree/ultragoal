use super::{inventory_checks, push};
use crate::scheduler::{SchedulerConfig, TaskClass};
use crate::schema_catalog::SchemaStore;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

type Failures = BTreeMap<String, Vec<String>>;
type PackageCheckTask = Box<dyn FnOnce() -> Failures + Send>;

pub(super) fn package_checks(
    root: &Path,
    store: &SchemaStore,
    check_ids: &[String],
    scheduler: SchedulerConfig,
    failures: &mut Failures,
) -> Vec<crate::scheduler::Metrics> {
    let tasks = package_check_tasks(
        Arc::new(root.to_path_buf()),
        Arc::new(store.clone()),
        Arc::new(check_ids.to_vec()),
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
                seed_check_context(out, &check_ids);
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
    ]
}

fn task(f: impl FnOnce(&mut Failures) + Send + 'static) -> PackageCheckTask {
    Box::new(move || {
        let mut out = BTreeMap::new();
        f(&mut out);
        out
    })
}

fn seed_check_context(failures: &mut Failures, check_ids: &[String]) {
    for id in check_ids {
        failures.entry(id.clone()).or_default();
    }
}

fn merge_failures(failures: &mut Failures, source: Failures) {
    for (check, items) in source {
        failures.entry(check).or_default().extend(items);
    }
}
