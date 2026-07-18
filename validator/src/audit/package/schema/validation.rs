use crate::json_boundary;
use crate::scheduler::{SchedulerConfig, TaskClass};
use crate::schema_catalog::{self, SchemaStore};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

pub(crate) struct Results {
    pub(crate) failures: Vec<String>,
}

pub(crate) fn mapped(root: &Path, store: &SchemaStore, scheduler: SchedulerConfig) -> Results {
    let mut failures = BTreeMap::from([("schema-valid".to_string(), Vec::new())]);
    validate_mapped(root, store, scheduler, &mut failures);
    Results {
        failures: failures.remove("schema-valid").unwrap_or_default(),
    }
}

fn validate_mapped(
    root: &Path,
    store: &SchemaStore,
    scheduler: SchedulerConfig,
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    let mut mapped = super::map::base();
    add_mapped_globs(root, &mut mapped);
    scheduled_checks(root, store, scheduler, mapped, failures)
}

fn scheduled_checks(
    root: &Path,
    store: &SchemaStore,
    scheduler: SchedulerConfig,
    mapped: BTreeMap<String, &'static str>,
    failures: &mut BTreeMap<String, Vec<String>>,
) {
    let root = Arc::new(root.to_path_buf());
    let store = Arc::new(store.clone());
    let tasks = mapped
        .into_iter()
        .map(|(path, schema)| {
            let root = Arc::clone(&root);
            let store = Arc::clone(&store);
            Box::new(move || match json_boundary::read_json(&root.join(&path)) {
                Ok(value) => schema_catalog::schema_errors(store.as_ref(), schema, &value)
                    .first()
                    .map(|error| format!("{path}: {error}")),
                Err(error) => Some(format!("{path}: json_load_failed: {error}")),
            }) as Box<dyn FnOnce() -> Option<String> + Send>
        })
        .collect::<Vec<_>>();
    let scheduled = crate::scheduler::run_ordered(scheduler, TaskClass::PureReadParallel, tasks);
    for failure in scheduled.values.into_iter().flatten() {
        failures
            .entry("schema-valid".to_string())
            .or_default()
            .push(failure);
    }
}

fn add_mapped_globs(root: &Path, mapped: &mut BTreeMap<String, &'static str>) {
    for (directory, schema) in [
        ("fixtures/valid", "fixture-bundle.schema.json"),
        ("fixtures/red", "red-packet.schema.json"),
        (
            "fixtures/mandatory-law-surfaces/valid",
            "mandatory-law-surface-receipt.schema.json",
        ),
    ] {
        if let Ok(entries) = std::fs::read_dir(root.join(directory)) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|extension| extension.to_str()) == Some("json")
                    && let Ok(relative) = path.strip_prefix(root)
                {
                    mapped.insert(relative.to_string_lossy().replace('\\', "/"), schema);
                }
            }
        }
    }
}
