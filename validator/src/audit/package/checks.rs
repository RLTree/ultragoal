use crate::json_boundary;
use crate::scheduler::{SchedulerConfig, TaskClass};
use crate::schema_catalog::{self, SchemaStore};
use crate::target_fixtures;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

pub struct CheckResults {
    pub failures: BTreeMap<String, Vec<String>>,
    pub scheduler_metrics: Vec<crate::scheduler::Metrics>,
}

#[cfg(test)]
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
    inventory_checks(root, &mut failures);
    crate::audit::red::catalog::check(root, store, &mut failures);
    crate::audit::package::text_checks::run(root, store, check_ids, &mut failures);
    crate::audit::review_history::check(root, &mut failures);
    failures
        .entry("research-source-authority-article-to-law-integration".to_string())
        .or_default()
        .extend(crate::audit::research::failures(root));
    failures
        .entry("harness-improvement-loop-trace-feedback-eval-codex-handoff".to_string())
        .or_default()
        .extend(crate::audit::improvement_loop::package_failures(root));
    for failure in crate::review::round::fixture_failures(root) {
        push(&mut failures, "validator-execution-provenance", failure);
    }
    for failure in crate::review::materiality::fixture_failures(root) {
        push(&mut failures, "material-review-scope-gate", failure);
    }
    failures
        .entry("target-repo-audit-capability".to_string())
        .or_default()
        .extend(target_fixtures::target_capability_failures(
            root,
            validator_artifacts,
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

fn mapped_schema_checks(
    root: &Path,
    store: &SchemaStore,
    scheduler: SchedulerConfig,
    failures: &mut BTreeMap<String, Vec<String>>,
) -> Vec<crate::scheduler::Metrics> {
    let mut mapped = crate::audit::package::schema::map::base();
    add_mapped_globs(root, &mut mapped);
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
    match json_boundary::read_json(&root.join("plugin-manifest-draft.json")) {
        Ok(manifest) => {
            for failure in crate::package::inventory::inventory_closure_failures(root, &manifest) {
                let check = if failure.contains("duplicates=") {
                    "plugin-inventory-exactly-once"
                } else {
                    "plugin-inventory-closure"
                };
                push(failures, check, failure);
            }
            for failure in crate::skill_links::manifest_failures(root, &manifest) {
                push(failures, "skill-inventory-closure", failure.detail);
            }
            for failure in crate::package::resource::purpose::failures(root, &manifest) {
                push(
                    failures,
                    "plugin-inventory-closure",
                    format!("{}: {}", failure.code, failure.detail),
                );
            }
            for failure in crate::audit::namespace::law::package_failures(root, &manifest) {
                push(failures, "namespace-progressive-disclosure", failure);
            }
        }
        Err(err) => push(
            failures,
            "plugin-inventory-closure",
            format!("plugin-manifest-draft.json load failed: {err}"),
        ),
    }
}

fn push(failures: &mut BTreeMap<String, Vec<String>>, check: &str, detail: impl Into<String>) {
    failures
        .entry(check.to_string())
        .or_default()
        .push(detail.into());
}
