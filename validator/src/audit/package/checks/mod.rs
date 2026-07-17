use crate::json_boundary;
use crate::scheduler::{SchedulerConfig, TaskClass};
use crate::schema_catalog::{self, SchemaStore};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

mod scheduled;

pub struct CheckResults {
    pub failures: BTreeMap<String, Vec<String>>,
    pub scheduler_metrics: Vec<crate::scheduler::Metrics>,
}

pub struct SchemaValidationResults {
    pub failures: Vec<String>,
    pub scheduler_metrics: Vec<crate::scheduler::Metrics>,
}

pub fn checks(
    root: &Path,
    store: &SchemaStore,
    check_ids: &[String],
    validator_artifacts: &[Value],
) -> BTreeMap<String, Vec<String>> {
    checks_with_scheduler(
        root,
        store,
        check_ids,
        validator_artifacts,
        SchedulerConfig::from_jobs(None).expect("default scheduler"),
    )
    .failures
}

pub fn checks_with_scheduler(
    root: &Path,
    store: &SchemaStore,
    check_ids: &[String],
    validator_artifacts: &[Value],
    scheduler: SchedulerConfig,
) -> CheckResults {
    let mut failures = check_ids
        .iter()
        .map(|id| (id.clone(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    let mut scheduler_metrics = Vec::new();
    scheduler_metrics.extend(mapped_schema_checks(root, store, scheduler, &mut failures));
    scheduler_metrics.extend(scheduled::package_checks(
        root,
        store,
        check_ids,
        validator_artifacts,
        scheduler,
        &mut failures,
    ));
    CheckResults {
        failures,
        scheduler_metrics,
    }
}

pub fn final_hygiene_check(root: &Path, failures: &mut BTreeMap<String, Vec<String>>) {
    let bytecode = crate::package::inventory::final_bytecode_failures(root);
    if !bytecode.is_empty() {
        failures
            .entry("plugin-inventory-closure".to_string())
            .or_default()
            .extend(bytecode);
    }
}

pub fn schema_validation_results(
    root: &Path,
    store: &SchemaStore,
    scheduler: SchedulerConfig,
) -> SchemaValidationResults {
    let mut failures = BTreeMap::from([("schema-valid".to_string(), Vec::new())]);
    let scheduler_metrics = mapped_schema_checks(root, store, scheduler, &mut failures);
    SchemaValidationResults {
        failures: failures.remove("schema-valid").unwrap_or_default(),
        scheduler_metrics,
    }
}

pub fn schema_validation_results_for_paths(
    root: &Path,
    store: &SchemaStore,
    scheduler: SchedulerConfig,
    paths: &[String],
) -> SchemaValidationResults {
    let mut failures = BTreeMap::from([("schema-valid".to_string(), Vec::new())]);
    let scheduler_metrics =
        mapped_schema_checks_for_paths(root, store, scheduler, paths, &mut failures);
    SchemaValidationResults {
        failures: failures.remove("schema-valid").unwrap_or_default(),
        scheduler_metrics,
    }
}

fn mapped_schema_checks(
    root: &Path,
    store: &SchemaStore,
    scheduler: SchedulerConfig,
    failures: &mut BTreeMap<String, Vec<String>>,
) -> Vec<crate::scheduler::Metrics> {
    let mut mapped = crate::audit::package::schema::map::base();
    add_mapped_globs(root, &mut mapped);
    scheduled_mapped_schema_checks(root, store, scheduler, mapped, failures)
}

fn mapped_schema_checks_for_paths(
    root: &Path,
    store: &SchemaStore,
    scheduler: SchedulerConfig,
    paths: &[String],
    failures: &mut BTreeMap<String, Vec<String>>,
) -> Vec<crate::scheduler::Metrics> {
    if paths.is_empty() {
        return Vec::new();
    }
    let mut mapped = crate::audit::package::schema::map::base();
    add_mapped_globs(root, &mut mapped);
    let mapped = paths
        .iter()
        .filter_map(|rel| mapped.get(rel).copied().map(|schema| (rel.clone(), schema)))
        .collect::<BTreeMap<_, _>>();
    scheduled_mapped_schema_checks(root, store, scheduler, mapped, failures)
}

fn scheduled_mapped_schema_checks(
    root: &Path,
    store: &SchemaStore,
    scheduler: SchedulerConfig,
    mapped: BTreeMap<String, &'static str>,
    failures: &mut BTreeMap<String, Vec<String>>,
) -> Vec<crate::scheduler::Metrics> {
    let root = Arc::new(root.to_path_buf());
    let store = Arc::new(store.clone());
    let tasks = mapped
        .into_iter()
        .map(|(rel, schema)| {
            let root = Arc::clone(&root);
            let store = Arc::clone(&store);
            Box::new(move || match json_boundary::read_json(&root.join(&rel)) {
                Ok(value) => {
                    if let Some(first) =
                        schema_catalog::schema_errors(store.as_ref(), schema, &value).first()
                    {
                        Some(format!("{rel}: {first}"))
                    } else {
                        None
                    }
                }
                Err(err) => Some(format!("{rel}: json_load_failed: {err}")),
            }) as Box<dyn FnOnce() -> Option<String> + Send>
        })
        .collect::<Vec<_>>();
    let scheduled = crate::scheduler::run_ordered(scheduler, TaskClass::PureReadParallel, tasks);
    for failure in scheduled.values.into_iter().flatten() {
        push(failures, "schema-valid", failure);
    }
    vec![scheduled.metrics]
}

fn add_mapped_globs(root: &Path, mapped: &mut BTreeMap<String, &'static str>) {
    add_glob(root, "fixtures/valid", "fixture-bundle.schema.json", mapped);
    add_glob(root, "fixtures/red", "red-packet.schema.json", mapped);
    add_glob(
        root,
        "fixtures/review-round/valid",
        "review-round-receipt.schema.json",
        mapped,
    );
    add_glob(
        root,
        "fixtures/review-materiality/valid",
        "review-materiality-gate.schema.json",
        mapped,
    );
    add_glob(
        root,
        "fixtures/mandatory-law-surfaces/valid",
        "mandatory-law-surface-receipt.schema.json",
        mapped,
    );
}

fn add_glob(
    root: &Path,
    dir: &str,
    schema: &'static str,
    mapped: &mut BTreeMap<String, &'static str>,
) {
    if let Ok(entries) = std::fs::read_dir(root.join(dir)) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && let Ok(rel) = path.strip_prefix(root)
            {
                mapped.insert(rel.to_string_lossy().replace('\\', "/"), schema);
            }
        }
    }
}

fn inventory_checks(root: &Path, failures: &mut BTreeMap<String, Vec<String>>) {
    crate::audit::cli::self_law::append_inventory_checks(root, failures);
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, check: &str, detail: impl Into<String>) {
    failures
        .entry(check.to_string())
        .or_default()
        .push(detail.into());
}
